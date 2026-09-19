// ==============================================================================
// BAPU STUDIO — REAL POSTGRESQL INTEGRATION TEST
// ==============================================================================
// NOT part of `npm test` / CI — this downloads and boots a real ~40MB PostgreSQL
// binary via `embedded-postgres`, which is too slow/heavy to run on every push
// across a 9-way OS/Node CI matrix. Run explicitly: `npm run test:postgres`.
//
// This exercises the actual `pg` driver (a real dependency of the app) against a
// real PostgreSQL server, using the same connection-config and schema-introspection
// logic electron-main.cjs uses for db:test-connection / db:query / db:get-schema —
// copied here verbatim rather than imported, since electron-main.cjs does
// `require('electron')` at module load and can't be required outside a real
// Electron process.

import assert from 'node:assert';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import pg from 'pg';
import EmbeddedPostgresModule from 'embedded-postgres';

const EmbeddedPostgres = EmbeddedPostgresModule.default || EmbeddedPostgresModule;
const __dirname = path.dirname(fileURLToPath(import.meta.url));

console.log('🐘 Starting Real PostgreSQL Integration Test Suite for Bapu Studio...\n');

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

// ---- verbatim copy of getPgConfig() from electron-main.cjs ----
function getPgConfig(config, connectOverride) {
  let host = config.host;
  if (!host && config.connectionString) {
    try {
      const match = config.connectionString.match(/@([^/:?]+)/);
      if (match) host = match[1];
    } catch {}
  }

  const isSslNeeded = config.ssl !== false && (
    config.ssl === true ||
    Boolean(config.sslCaCert || config.sslClientCert) ||
    (config.connectionString && (config.connectionString.includes('sslmode') || config.connectionString.includes('neon.tech') || config.connectionString.includes('supabase') || config.connectionString.includes('aiven') || config.connectionString.includes('render.com') || config.connectionString.includes('aws'))) ||
    (host && !host.includes('localhost') && !host.includes('127.0.0.1'))
  );

  let sslConfig = undefined;
  if (isSslNeeded) {
    sslConfig = {
      rejectUnauthorized: config.sslRejectUnauthorized !== undefined ? Boolean(config.sslRejectUnauthorized) : (config.sslCaCert ? true : false),
      servername: host || undefined
    };
    if (config.sslCaCert) sslConfig.ca = config.sslCaCert;
    if (config.sslClientCert) sslConfig.cert = config.sslClientCert;
    if (config.sslClientKey) sslConfig.key = config.sslClientKey;
  }

  if (config.connectionString) {
    return { connectionString: config.connectionString, ssl: sslConfig, connectionTimeoutMillis: 10000 };
  }

  return {
    host: (connectOverride ? connectOverride.host : host) || 'localhost',
    port: connectOverride ? connectOverride.port : (parseInt(config.port, 10) || 5432),
    database: config.database || 'postgres',
    user: config.username || 'postgres',
    password: config.password || '',
    connectionTimeoutMillis: 10000,
    ssl: sslConfig
  };
}

const PG_PORT = 15544;
const DATA_DIR = path.join(__dirname, '.tmp-pg-test-data');
fs.rmSync(DATA_DIR, { recursive: true, force: true });

const pgServer = new EmbeddedPostgres({
  databaseDir: DATA_DIR,
  user: 'postgres',
  password: 'postgres',
  port: PG_PORT,
  persistent: false
});

console.log(`Booting a real embedded PostgreSQL server on 127.0.0.1:${PG_PORT}...`);
await pgServer.initialise();
await pgServer.start();
console.log('Postgres ready.\n');

const baseConfig = {
  host: '127.0.0.1',
  port: String(PG_PORT),
  database: 'postgres',
  username: 'postgres',
  password: 'postgres',
  ssl: false
};

try {
  await test('getPgConfig() builds a working config — real connect + SELECT 1', async () => {
    const client = new pg.Client(getPgConfig(baseConfig));
    await client.connect();
    const res = await client.query('SELECT 1 as one');
    assert.strictEqual(res.rows[0].one, 1);
    await client.end();
  });

  await test('Real schema creation + information_schema introspection (mirrors db:get-schema)', async () => {
    const client = new pg.Client(getPgConfig(baseConfig));
    await client.connect();
    await client.query(`
      CREATE TABLE IF NOT EXISTS accounts (id SERIAL PRIMARY KEY, email TEXT NOT NULL, balance_cents INTEGER DEFAULT 0);
      INSERT INTO accounts (email, balance_cents) VALUES ('a@b.com', 500), ('c@d.com', 1200);
    `);

    const tableRes = await client.query(`
      SELECT table_name FROM information_schema.tables
      WHERE table_schema = 'public' AND table_type = 'BASE TABLE'
        AND table_name NOT LIKE 'pg_%' AND table_name NOT LIKE 'sql_%'
      ORDER BY table_name;
    `);
    assert.ok(tableRes.rows.some(r => r.table_name === 'accounts'), 'accounts table should be visible via information_schema');

    const colRes = await client.query(`
      SELECT column_name, data_type, is_nullable FROM information_schema.columns
      WHERE table_name = $1 AND table_schema = 'public';
    `, ['accounts']);
    const emailCol = colRes.rows.find(c => c.column_name === 'email');
    assert.strictEqual(emailCol.is_nullable, 'NO', 'NOT NULL columns should report is_nullable = NO');

    const countRes = await client.query('SELECT COUNT(*) FROM accounts;');
    assert.strictEqual(Number(countRes.rows[0].count), 2);

    await client.end();
  });

  await test('Real query execution returns correct columns/rows for the DataGrid', async () => {
    const client = new pg.Client(getPgConfig(baseConfig));
    await client.connect();
    const res = await client.query('SELECT email, balance_cents FROM accounts ORDER BY balance_cents DESC;');
    assert.strictEqual(res.fields.map(f => f.name).join(','), 'email,balance_cents');
    assert.strictEqual(res.rows[0].email, 'c@d.com');
    await client.end();
  });

  await test('Real EXPLAIN ANALYZE returns a genuine query plan (mirrors wrapExplainQuery)', async () => {
    const client = new pg.Client(getPgConfig(baseConfig));
    await client.connect();
    const res = await client.query('EXPLAIN (ANALYZE, COSTS, VERBOSE, BUFFERS)\nSELECT * FROM accounts;');
    const planText = res.rows.map(r => Object.values(r)[0]).join('\n');
    assert.ok(/Scan/.test(planText), 'should contain a real Postgres plan node (e.g. Seq Scan)');
    assert.ok(/actual time/.test(planText), 'ANALYZE should include real actual-time timing, not a canned string');
    await client.end();
  });

  await test('Wrong password produces a real authentication error, not a false success', async () => {
    const client = new pg.Client(getPgConfig({ ...baseConfig, password: 'definitely-wrong' }));
    let threw = false;
    try {
      await client.connect();
      await client.end();
    } catch (err) {
      threw = true;
      assert.ok(/password/i.test(err.message), `expected a password-related error, got: ${err.message}`);
    }
    assert.ok(threw, 'connecting with the wrong password must fail, not silently succeed');
  });
} finally {
  await pgServer.stop();
  fs.rmSync(DATA_DIR, { recursive: true, force: true });
  console.log('\nStopped embedded PostgreSQL server and cleaned up its data directory.');
}

console.log('\n==============================================================================');
console.log(`📊 POSTGRES INTEGRATION SUITE: ${passedTests} Passed, ${failedTests} Failed.`);
console.log('==============================================================================\n');

if (failedTests > 0) {
  process.exit(1);
}
