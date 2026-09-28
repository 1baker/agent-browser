import assert from 'node:assert/strict';
import { temporaryRoute, quote, connectionSql, selectedTemporaryLabel, selectTemporaryEntry } from './lib/temporary-rdp-route.js';
for (const label of ['', 'A', 'B', 'c', 'CC', '../C', 'C;']) assert.throws(() => temporaryRoute(label));
for (const label of ['C', 'D', 'Z']) {
  const route = temporaryRoute(label);
  const sql = connectionSql(route, 'a'.repeat(64), 'bak3r');
  assert.match(sql, /BEGIN;/);
  assert.match(sql, /pg_advisory_xact_lock/);
  assert.match(sql, /temporary route name already exists/);
  assert.match(sql, /operator Guacamole identity missing/);
  assert.match(sql, /COMMIT;/);
  assert.doesNotMatch(sql, /UPDATE|DELETE|DROP|TRUNCATE|agent-browser-rdp-[ab]'/);
  assert.match(sql, /disable-copy/);
  assert.match(sql, /disable-paste/);
  assert.match(sql, /entity.type = 'USER'/);
}
assert.equal(quote("a'b"), "'a''b'");
assert.throws(() => quote('a\\b'));
assert.throws(() => connectionSql(temporaryRoute('C'), 'bad', 'bak3r'));
assert.throws(() => connectionSql(temporaryRoute('C'), 'a'.repeat(64), "x';"));
const c = { id: 'guacamole-rdp-c', connectionName: temporaryRoute('C').name,
  frameUrl: 'http://127.0.0.1:8092/guacamole/#/client/MwBjAHBvc3RncmVzcWw=',
  connectionId: '3', routeId: 'guacamole:3', target: { routeUser: temporaryRoute('C').user } };
assert.equal(selectedTemporaryLabel([]), null);
assert.equal(selectedTemporaryLabel(['--route-label', 'C']), 'C');
assert.throws(() => selectedTemporaryLabel(['--route-label']));
assert.throws(() => selectedTemporaryLabel(['--route-label', 'A']));
assert.deepEqual(selectTemporaryEntry([{id:'guacamole-rdp-b'}, c, {id:'guacamole-rdp-a'}], 'C'), [c]);
assert.throws(() => selectTemporaryEntry([c, c], 'C'));
assert.throws(() => selectTemporaryEntry([], 'C'));
assert.throws(() => selectTemporaryEntry([{...c, routeId:'guacamole:1'}], 'C'));
assert.throws(() => selectTemporaryEntry([{...c, routeDescriptor:{localEmbedUrl:'http://127.0.0.1:8092/guacamole/#/client/MQBjAHBvc3RncmVzcWw='}}], 'C'));
assert.throws(() => selectTemporaryEntry([{...c, externalUrl:'https://example.com/guacamole/#/client/MQBjAHBvc3RncmVzcWw='}], 'C'));
assert.throws(() => selectTemporaryEntry([{...c, frameUrl:c.frameUrl.replace('127.0.0.1', 'example.com')}], 'C'));
console.log('temporary route SQL isolation and input tests passed');
