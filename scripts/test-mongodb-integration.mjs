// ==============================================================================
// BAPU STUDIO — REAL MONGODB INTEGRATION TEST
// ==============================================================================
// NOT part of `npm test` / CI, same reasoning as the other test-*-integration.mjs
// scripts — downloads and boots a real MongoDB binary via `mongodb-memory-server`,
// too slow/heavy for a 9-way OS/Node CI matrix. Run explicitly: `npm run test:mongodb`.
//
// Exercises the real `mongodb` driver (a real dependency of the app) against a real
// mongod process, mirroring the exact query-parsing and schema-introspection logic
// electron-main.cjs uses for db:query / db:get-schema — copied here verbatim rather
// than imported, since electron-main.cjs does `require('electron')` at module load
// and can't be required outside a real Electron process.

import assert from 'node:assert';
import { MongoClient } from 'mongodb';
import { MongoMemoryServer } from 'mongodb-memory-server';

console.log('🍃 Starting Real MongoDB Integration Test Suite for Bapu Studio...\n');

let passedTests = 0;
let failedTests = 0;

async function test(name, fn) {
  try {
    await fn();
    console.log(`  ✅ PASS: ${name}`);
    passedTests++;
  } catch (err) {
    console.error(`  ❌ FAIL: ${name}`);
    console.error(`     Error: ${err.message}`);
    failedTests++;
  }
}

// ---- verbatim copy of the query-string parsing logic from electron-main.cjs's db:query ----
function parseMongoQuery(sql, defaultCollectionName) {
  let collectionName = defaultCollectionName;
  let filter = {};

  const trimmed = sql.trim();
  if (trimmed.startsWith('{') && trimmed.endsWith('}')) {
    try { filter = JSON.parse(trimmed); } catch {}
  } else {
    const match = trimmed.match(/^([a-zA-Z0-9_-]+)\.find\((.*)\)/);
    if (match) {
      collectionName = match[1];
      try { filter = JSON.parse(match[2] || '{}'); } catch {}
    }
  }
  return { collectionName, filter };
}

console.log('Booting a real MongoDB server (may download the binary on first run)...');
const mongod = await MongoMemoryServer.create();
const uri = mongod.getUri();
console.log(`MongoDB ready at ${uri}\n`);

const client = new MongoClient(uri);

try {
  await test('Real connect + ping (mirrors db:test-connection)', async () => {
    await client.connect();
    const pingResult = await client.db('admin').command({ ping: 1 });
    assert.strictEqual(pingResult.ok, 1);
  });

  await test('JSON-filter query format: {"status":"active"} parses and matches real documents', async () => {
    const db = client.db('testdb');
    await db.collection('users').insertMany([
      { name: 'Alex', status: 'active' },
      { name: 'Sam', status: 'inactive' },
      { name: 'Priya', status: 'active' }
    ]);

    const { collectionName, filter } = parseMongoQuery('{"status":"active"}', 'documents');
    assert.strictEqual(collectionName, 'documents', 'a bare JSON filter should keep the default collection name');
    const docs = await db.collection('users').find(filter).limit(50).toArray();
    // The default collection name isn't "users" here since a bare JSON filter doesn't name one —
    // this mirrors the app's real behavior: the UI supplies the collection via config.tables[0].
    const usersMatching = await db.collection('users').find(filter).toArray();
    assert.strictEqual(usersMatching.length, 2, 'exactly the 2 active users should match');
  });

  await test('"collection.find({...})" string format: real regex parsing + real query execution', async () => {
    const db = client.db('testdb');
    const { collectionName, filter } = parseMongoQuery("users.find({ \"status\": \"active\" })", 'documents');
    assert.strictEqual(collectionName, 'users');
    assert.deepStrictEqual(filter, { status: 'active' });

    const docs = await db.collection(collectionName).find(filter).limit(50).toArray();
    assert.strictEqual(docs.length, 2);
    assert.ok(docs.every(d => d.status === 'active'));
  });

  await test('Real schema introspection: listCollections + countDocuments + sample doc field extraction', async () => {
    const db = client.db('testdb');
    await db.collection('orders').insertMany([
      { customer: 'a@b.com', total: 19.99, tags: ['x'] },
      { customer: 'c@d.com', total: 42.5, tags: ['y', 'z'] }
    ]);

    const collections = await db.listCollections().toArray();
    const names = collections.map(c => c.name);
    assert.ok(names.includes('users') && names.includes('orders'));

    const count = await db.collection('orders').countDocuments();
    assert.strictEqual(count, 2);

    const sampleDoc = await db.collection('orders').findOne({});
    const columns = Object.keys(sampleDoc).map(k => ({
      name: k,
      type: typeof sampleDoc[k] === 'object' ? 'OBJECT' : typeof sampleDoc[k]
    }));
    assert.ok(columns.some(c => c.name === '_id'));
    assert.ok(columns.some(c => c.name === 'customer' && c.type === 'string'));
    assert.ok(columns.some(c => c.name === 'total' && c.type === 'number'));
    assert.ok(columns.some(c => c.name === 'tags' && c.type === 'OBJECT'), 'arrays should be classified as OBJECT, matching the app logic');
  });

  await test('ObjectId serialization for UI display (real ObjectId, not a string stand-in)', async () => {
    const db = client.db('testdb');
    const insertResult = await db.collection('widgets').insertOne({ name: 'Widget A' });
    const doc = await db.collection('widgets').findOne({ _id: insertResult.insertedId });

    assert.strictEqual(typeof doc._id, 'object', 'a real ObjectId should be an object, not already a string');
    const serialized = { ...doc, _id: String(doc._id) };
    assert.strictEqual(typeof serialized._id, 'string');
    assert.strictEqual(serialized._id, insertResult.insertedId.toString());
  });

  await test('Malformed filter JSON degrades to an empty filter rather than throwing (mirrors real try/catch)', async () => {
    const { filter } = parseMongoQuery('{not valid json', 'documents');
    assert.deepStrictEqual(filter, {}, 'unparseable filter JSON should fall back to {} instead of crashing the query');
  });
} finally {
  await client.close();
  await mongod.stop();
  console.log('\nStopped the embedded MongoDB server.');
}

console.log('\n==============================================================================');
console.log(`📊 MONGODB INTEGRATION SUITE: ${passedTests} Passed, ${failedTests} Failed.`);
console.log('==============================================================================\n');

if (failedTests > 0) {
  process.exit(1);
}
