// Development compatibility reference only. Mermaid is never shipped in our CLI/WASM.
import assert from 'node:assert/strict';
import { mkdir, writeFile } from 'node:fs/promises';
import { withBrowser } from './browser-harness.mjs';
import { execFileSync } from 'node:child_process';
const cases = JSON.parse(execFileSync('cargo', ['run', '--quiet', '--locked', '-p', 'diagram-render', '--example', 'reference'], { encoding: 'utf8' }));
const results = [];
await withBrowser(async page => {
  for (const [name, [source, native]] of Object.entries(cases)) {
    const result = await page.evaluate(async ({ source, name, family }) => {
      const { default: mermaid } = await import('/node_modules/mermaid/dist/mermaid.esm.min.mjs');
      mermaid.initialize({ startOnLoad: false, securityLevel: 'strict', flowchart: { htmlLabels: false } });
      const diagram = await mermaid.mermaidAPI.getDiagramFromText(source);
      if (source.trimStart().startsWith('sequenceDiagram')) {
        const actors = [...diagram.db.getActors()].map(([id,p]) => ({id, label:p.description, kind:p.type}));
        const messages = diagram.db.getMessages().filter(m => [0,1,3,4,5,6].includes(m.type)).map(m => ({source:m.from,target:m.to,label:m.message,type:m.type}));
        const svg=(await mermaid.render(`ref-${name}`,source)).svg;
        return {actors,messages,rendered:svg.includes('<svg')};
      }
      if (family && family !== 'sequence') {
        const array=v=>v instanceof Map?[...v.values()]:Array.isArray(v)?v:Object.values(v);
        const svg=(await mermaid.render(`ref-${name}`,source)).svg;
        if(family==='class') return {classes:array(diagram.db.getClasses()).map(c=>({id:c.id,label:c.label,annotations:c.annotations,attributes:c.members.length,methods:c.methods.length})),relations:diagram.db.getRelations(),rendered:svg.includes('<svg')};
        if(family==='state') return {states:array(diagram.db.getStates()).map(s=>({id:s.id,kind:s.type})),relations:diagram.db.getRelations(),rendered:svg.includes('<svg')};
        if(family==='er') {const entries=[...diagram.db.getEntities()];const ids=Object.fromEntries(entries.map(([name,e])=>[e.id,name]));return {entities:entries.map(([id,e])=>({id,attributes:e.attributes})),relations:diagram.db.getRelationships().map(r=>({source:ids[r.entityA],target:ids[r.entityB],label:r.roleA,start:r.relSpec.cardB,end:r.relSpec.cardA,identifying:r.relSpec.relType==='IDENTIFYING'})),rendered:svg.includes('<svg')};}
        if(family==='gantt') return {tasks:diagram.db.getTasks().map(t=>({id:t.id,label:t.task.trim(),section:t.section,start:t.startTime.toISOString().slice(0,10),end:t.endTime.toISOString().slice(0,10),done:!!t.done,active:!!t.active,critical:!!t.crit,milestone:!!t.milestone})),rendered:svg.includes('<svg')};
      }
      const rawVertices = diagram.db.getVertices();
      const vertices = rawVertices instanceof Map ? [...rawVertices.values()] : Object.values(rawVertices);
      const edges = diagram.db.getEdges();
      const reference = vertices.map(v => ({ id: v.id, shape: v.type || 'square', label: String(v.text).replace(/<br\s*\/?\s*>/g, '\n') }));
      const svg = (await mermaid.render(`ref-${name}`, source)).svg;
      return { reference, edges: edges.map(e => ({ source: e.start, target: e.end, arrow: e.type, stroke: e.stroke, span: e.length })), rendered: svg.includes('<svg') };
    }, { source, name, family:native.family });
    if (native.family === 'sequence') {
      const seq=native.sequence;
      assert.deepEqual(seq.participants.map(p => ({id:p.id,label:p.label,kind:p.kind.toLowerCase()})),result.actors,`participants: ${name}`);
      const types={SolidArrow:0,DottedArrow:1,SolidCross:3,DottedCross:4,SolidOpen:5,DottedOpen:6};
      assert.deepEqual(seq.events.filter(e => e.Message).map(({Message:m}) => ({source:seq.participants[m.source].id,target:seq.participants[m.target].id,label:m.label,type:types[m.kind]})),result.messages,`messages: ${name}`);
      assert.equal(result.rendered,true);results.push({fixture:name,...result});continue;
    }
    if (native.family && native.family !== 'sequence') {
      const d=native.diagram;
      if(native.family==='class') {
        assert.deepEqual(d.classes.map(c=>({id:c.id,label:c.label,annotations:c.annotation?[c.annotation]:[],attributes:c.attributes.length,methods:c.methods.length})),result.classes,`class compartments: ${name}`);
        const marker={Inheritance:1,Realization:1,Composition:2,Aggregation:0,Association:3,Dependency:3,Link:'none'};
        assert.deepEqual(d.relations.map(r=>({source:d.classes[r.source].id,target:d.classes[r.target].id,start:r.reversed?marker[r.kind]:'none',end:r.reversed?'none':marker[r.kind],dashed:r.dashed,label:r.label,startLabel:r.source_cardinality,endLabel:r.target_cardinality})),result.relations.map(r=>({source:r.id1,target:r.id2,start:r.relation.type1,end:r.relation.type2,dashed:r.relation.lineType===1,label:r.title||'',startLabel:r.relationTitle1==='none'?'':r.relationTitle1,endLabel:r.relationTitle2==='none'?'':r.relationTitle2})),`class relations: ${name}`);
      }
      if(native.family==='state') {
        const id=s=>s.kind==='Initial'?'root_start':s.kind==='Final'?'root_end':s.id;
        assert.deepEqual(d.states.map(id).sort(),result.states.map(s=>s.id).sort(),`state IDs: ${name}`);
        assert.deepEqual(d.transitions.map(t=>({source:id(d.states[t.source]),target:id(d.states[t.target]),label:t.label})),result.relations.map(r=>({source:r.id1,target:r.id2,label:r.relationTitle})),`transitions: ${name}`);
      }
      if(native.family==='er') {
        assert.deepEqual(d.entities.map(e=>({id:e.id,attributes:e.attributes.map(a=>({type:a.data_type,name:a.name,keys:a.keys,comment:a.comment}))})),result.entities,`ER attributes: ${name}`);
        const card={ExactlyOne:'ONLY_ONE',ZeroOrOne:'ZERO_OR_ONE',OneOrMore:'ONE_OR_MORE',ZeroOrMore:'ZERO_OR_MORE'};
        assert.deepEqual(d.relationships.map(r=>({source:d.entities[r.source].id,target:d.entities[r.target].id,label:r.label,start:card[r.source_cardinality],end:card[r.target_cardinality],identifying:r.identifying})),result.relations,`ER cardinalities: ${name}`);
      }
      if(native.family==='gantt') {
        // Stored dates are days since 0001-01-01; use UTC for deterministic comparison.
        const iso=day=>new Date((day-719162)*86400000).toISOString().slice(0,10);
        assert.deepEqual(d.tasks.map(t=>({id:t.id,label:t.label,section:t.section===null?'':d.sections[t.section],start:iso(t.start),end:iso(t.end),done:t.status==='Done',active:t.status==='Active',critical:t.critical,milestone:t.milestone})).sort((a,b)=>a.id.localeCompare(b.id)),result.tasks.sort((a,b)=>a.id.localeCompare(b.id)),`Gantt dates/status: ${name}`);
      }
      assert.equal(result.rendered,true);results.push({fixture:name,...result});continue;
    }
    const shapes = { Rectangle: 'square', Rounded: 'round', Diamond: 'diamond', Circle: 'circle', Stadium: 'stadium', Subroutine: 'subroutine', Cylinder: 'cylinder' };
    const strokes = { Solid: 'normal', Dotted: 'dotted', Thick: 'thick' };
    const ours = native.nodes.map(n => ({ id: n.id, shape: shapes[n.shape] })).sort((a, b) => a.id.localeCompare(b.id));
    assert.deepEqual(ours, result.reference.map(({ id, shape }) => ({ id, shape })).sort((a, b) => a.id.localeCompare(b.id)), `node IDs/shapes: ${name}`);
    assert.deepEqual(native.edges.map(e => ({ source: native.nodes[e.source].id, target: native.nodes[e.target].id, arrow: e.source_arrow ? 'double_arrow_point' : e.directed ? 'arrow_point' : 'arrow_open', stroke: strokes[e.kind], span: e.min_rank_span })), result.edges, `edge endpoints/arrowheads/strokes/rank spans: ${name}`);
    assert.equal(result.rendered, true);
    // Retain labels for human review: Mermaid stores entity encodings in its DB,
    // while diagram-rs normalizes them in the IR. Pixel equality is not expected.
    results.push({ fixture: name, ...result });
    if (results.length % 25 === 0) console.log(`Reference progress: ${results.length}/${Object.keys(cases).length}`);
  }
});
await mkdir('benchmarks/results', { recursive: true });
await writeFile('benchmarks/results/mermaid-reference.json', JSON.stringify({ mermaid: '11.12.0', results }, null, 2));
console.log(`Mermaid 11.12.0 reference: ${results.length} diagrams parsed/rendered across all six families; structural comparisons agree`);
