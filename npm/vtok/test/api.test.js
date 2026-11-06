const test = require('node:test');
const assert = require('node:assert/strict');
const vtok = require('../index.js');

const TS = `// auth
export class AuthMgr {
  constructor(private db: Db) {}
  validate(user: string, password: string): boolean {
    if (!user || !password) {
      return false;
    }
    return this.db.query(user, password);
  }
}
`;
const PY = `"""Pagination."""

def paginate(items, page, size):
    """One page."""
    if page < 1:
        raise ValueError("page must be >= 1")
    start = (page - 1) * size
    return list(items[start:start + size])
`;

test('languages', () => {
  const names = vtok.languages().map(l => l.name).sort();
  assert.deepEqual(names, ['python', 'typescript']);
});

test('encode / decode roundtrip (typescript)', () => {
  const r = vtok.encode('typescript', TS);
  assert.ok(r.vtok.length < TS.length);
  assert.ok(!r.vtok.includes('// auth'));
  const back = vtok.decode('typescript', r.vtok, r.table, r.literals);
  assert.match(back, /validate\(user:string,password:string\):boolean/);
});

test('python: compact notation and readable decode', () => {
  const r = vtok.encode('python', PY, { alias: false });
  assert.ok(r.vtok.includes('if page<1:raise ValueError'));
  assert.ok(!r.vtok.includes('Pagination'));
  const back = vtok.decode('python', r.vtok, r.table);
  assert.match(back, /def paginate\(items,page,size\):\n    if page<1:/);
});

test('hidden literals restore', () => {
  const src = "def f():\n    return 'a rather long error message that costs several tokens'\n";
  const r = vtok.encode('python', src, { hideLiterals: 5 });
  assert.ok(!r.vtok.includes('rather long'));
  assert.ok(Object.keys(r.literals).length === 1);
  assert.match(vtok.decode('python', r.vtok, r.table, r.literals), /rather long error message/);
});

test('views', () => {
  const sk = vtok.encode('typescript', TS, { view: 'skeleton' }).vtok;
  assert.ok(!sk.includes('query'));
  const lean = vtok.encode('typescript', TS, { view: 'lean' }).vtok;
  assert.ok(!lean.includes('string'));
});

test('split / chunk / apply', () => {
  let big = '';
  for (let i = 0; i < 40; i++) big += `def f${i}(x):\n    y = x + ${i}\n    return y * 2\n\n`;
  const s = vtok.split('python', big, 30);
  assert.ok(!s.wholeFile && s.chunks.length > 1);
  const id = s.chunks[0].id;
  const c = vtok.chunk('python', big, id, 30);
  const edited = c.replace('+0', '+100');
  assert.notEqual(edited, c);
  const out = vtok.apply('python', big, { [id]: edited }, 30);
  assert.ok(out.includes('y=x+100'));
  const same = vtok.apply('python', big, { [id]: c }, 30);
  assert.equal(same, big);
});

test('errors are exceptions', () => {
  assert.throws(() => vtok.encode('python', 'def (:'), /syntax/i);
  assert.throws(() => vtok.encode('cobol', 'x'), /unknown language/);
});

test('project table is shared', () => {
  const t = vtok.projectTable('python', [PY, PY.replace('paginate', 'paginate2')]);
  const r = vtok.encode('python', PY, { table: t });
  assert.equal(typeof r.vtok, 'string');
});
