// ==============================================================================
// BAPU STUDIO — REAL MYSQL INTEGRATION TEST
// ==============================================================================
// NOT part of `npm test` / CI, same reasoning as test-postgres-integration.mjs —
// downloads and boots a real MySQL binary via `mysql-memory-server`, too slow/heavy
// for a 9-way OS/Node CI matrix. Run explicitly: `npm run test:mysql`.
//
// Exercises the real `mysql2` driver (a real dependency of the app) against a real
// MySQL server, mirroring the exact SQL/config logic electron-main.cjs uses for
// db:test-connection / db:query / db:get-schema — copied here verbatim rather than
// imported, since electron-main.cjs does `require('electron')` at module load and
// can't be required outside a real Electron process.

import assert from 'node:assert';
import mysql from 'mysql2/promise';
import { createDB } from 'mysql-memory-server';

console.log('🐬 Starting Real MySQL Integration Test Suite for Bapu Studio...\n');

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

// ---- verbatim copy of getMysqlSslConfig() from electron-main.cjs ----
function getMysqlSslConfig(config) {
  const isSslNeeded = config.ssl === true || Boolean(config.sslCaCert || config.sslClientCert);
  if (!isSslNeeded) return undefined;

  const ssl = {
    rejectUnauthorized: config.sslRejectUnauthorized !== undefined ? Boolean(config.sslRejectUnauthorized) : (config.sslCaCert ? true : false)
  };
  if (config.sslCaCert) ssl.ca = config.sslCaCert;
  if (config.sslClientCert) ssl.cert = config.sslClientCert;
  if (config.sslClientKey) ssl.key = config.sslClientKey;
  return ssl;
}

console.log('Booting a real MySQL server (may download the binary on first run)...');
const db = await createDB();
console.log(`MySQL ready on 127.0.0.1:${db.port} (user: ${db.username}, db: ${db.dbName}).\n`);

const baseConfig = {
  host: '127.0.0.1',
  port: String(db.port),
  database: db.dbName,
  username: db.username,
  password: '',
  ssl: false
};

function buildConnectionOptions(config) {
  return {
    host: config.host || 'localhost',
    port: parseInt(config.port, 10) || 3306,
    database: config.database,
    user: config.username || 'root',
    password: config.password || '',
    connectTimeout: 8000,
    ssl: getMysqlSslConfig(config)
  };
}

try {
  await test('Real connect + SELECT 1 (mirrors db:test-connection)', async () => {
    const connection = await mysql.createConnection(buildConnectionOptions(baseConfig));
    const [rows] = await connection.query('SELECT 1 as one');
    assert.strictEqual(rows[0].one, 1);
    await connection.end();
  });

  await test('Real schema creation + information_schema/DESCRIBE introspection (mirrors db:get-schema)', async () => {
    const connection = await mysql.createConnection(buildConnectionOptions(baseConfig));
    await connection.query(`
      CREATE TABLE IF NOT EXISTS accounts (id INT AUTO_INCREMENT PRIMARY KEY, email VARCHAR(255) NOT NULL, balance_cents INT DEFAULT 0);
    `);
    await connection.query(`INSERT INTO accounts (email, balance_cents) VALUES ('a@b.com', 500), ('c@d.com', 1200);`);

    const [tableRows] = await connection.query(
      'SELECT table_name FROM information_schema.tables WHERE table_schema = ? ORDER BY table_name LIMIT 30;',
      [baseConfig.database]
    );
    const tableNames = tableRows.map(r => r.TABLE_NAME || r.table_name).filter(Boolean);
    assert.ok(tableNames.includes('accounts'), 'accounts table should be visible via information_schema');

    const [colRows] = await connection.query('DESCRIBE `accounts`');
    const emailCol = colRows.find(c => c.Field === 'email');
    assert.strictEqual(emailCol.Null, 'NO', 'NOT NULL columns should report Null = NO in DESCRIBE');
    const idCol = colRows.find(c => c.Field === 'id');
    assert.strictEqual(idCol.Key, 'PRI');

    const [countRows] = await connection.query('SELECT COUNT(*) as total FROM accounts;');
    assert.strictEqual(Number(countRows[0].total), 2);

    await connection.end();
  });

  await test('Real query execution returns correct columns/rows for the DataGrid', async () => {
    const connection = await mysql.createConnection(buildConnectionOptions(baseConfig));
    const [rows, fields] = await connection.query('SELECT email, balance_cents FROM accounts ORDER BY balance_cents DESC;');
    assert.strictEqual(fields.map(f => f.name).join(','), 'email,balance_cents');
    assert.strictEqual(rows[0].email, 'c@d.com');
    await connection.end();
  });

  await test('Real EXPLAIN FORMAT=JSON returns a genuine query plan (mirrors wrapExplainQuery)', async () => {
    const connection = await mysql.createConnection(buildConnectionOptions(baseConfig));
    const [rows] = await connection.query('EXPLAIN FORMAT=JSON\nSELECT * FROM accounts;');
    assert.ok(rows[0]?.EXPLAIN, 'EXPLAIN FORMAT=JSON should return an EXPLAIN column');
    // Don't assume a specific JSON schema version's key names (e.g. "query_block" vs. newer
    // "query_plan") — just confirm it's real, valid, non-trivial JSON referencing our table.
    const parsed = JSON.parse(rows[0].EXPLAIN);
    assert.ok(Object.keys(parsed).length > 0, 'parsed EXPLAIN JSON should not be empty');
    assert.ok(rows[0].EXPLAIN.includes('accounts'), 'the real plan should reference the real table name');
    await connection.end();
  });

  await test('Wrong password produces a real authentication error, not a false success', async () => {
    let threw = false;
    try {
      const connection = await mysql.createConnection(buildConnectionOptions({ ...baseConfig, password: 'definitely-wrong' }));
      await connection.end();
    } catch (err) {
      threw = true;
      assert.ok(/access denied/i.test(err.message), `expected an access-denied error, got: ${err.message}`);
    }
    assert.ok(threw, 'connecting with the wrong password must fail, not silently succeed');
  });
} finally {
  await db.stop();
  console.log('\nStopped the embedded MySQL server.');
}

console.log('\n==============================================================================');
console.log(`📊 MYSQL INTEGRATION SUITE: ${passedTests} Passed, ${failedTests} Failed.`);
console.log('==============================================================================\n');

if (failedTests > 0) {
  process.exit(1);
}
