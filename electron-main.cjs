const { app, BrowserWindow, ipcMain, shell, safeStorage, dialog } = require('electron');
const path = require('path');
const fs = require('fs/promises');
const net = require('net');
const pg = require('pg');
const mysql = require('mysql2/promise');
const { MongoClient, ObjectId } = require('mongodb');
const { Client: SshClient } = require('ssh2');
const Redis = require('ioredis');

// Active Connection Pools & Clients Cache
const pgPools = new Map();
const mysqlPools = new Map();
const mongoClients = new Map();
const redisClients = new Map();

const DEFAULT_DB_PORTS = { postgres: 5432, mysql: 3306, mongodb: 27017, redis: 6379 };
// SSH tunnels backing a pooled connection, keyed by config.id (same lifetime as the pool/client above)
const sshTunnels = new Map();

function createWindow() {
  const win = new BrowserWindow({
    width: 1320,
    height: 860,
    minWidth: 960,
    minHeight: 620,
    backgroundColor: '#090d14',
    title: 'Bapu Studio — Developer Cockpit',
    autoHideMenuBar: true,
    webPreferences: {
      nodeIntegration: false,
      contextIsolation: true,
      webSecurity: true,
      preload: path.join(__dirname, 'preload.cjs'),
    },
  });

  // Handle external links (e.g. GitHub Sponsor, Docs) to open in system default browser
  win.webContents.setWindowOpenHandler(({ url }) => {
    if (url.startsWith('https:') || url.startsWith('http:')) {
      shell.openExternal(url);
    }
    return { action: 'deny' };
  });

  win.webContents.on('will-navigate', (event, url) => {
    if (url.startsWith('http:') || url.startsWith('https:')) {
      const isDevServer = !app.isPackaged && url.includes('localhost:3000');
      if (!isDevServer) {
        event.preventDefault();
        shell.openExternal(url);
      }
    }
  });

  if (app.isPackaged) {
    win.loadFile(path.join(__dirname, 'dist', 'index.html'));
  } else {
    const devUrl = process.env.VITE_DEV_SERVER_URL || 'http://localhost:3000';
    win.loadURL(devUrl).catch(() => {
      win.loadFile(path.join(__dirname, 'dist', 'index.html'));
    });
  }

  // Set Content-Security-Policy headers
  const { session } = require('electron');
  session.defaultSession.webRequest.onHeadersReceived((details, callback) => {
    callback({
      responseHeaders: {
        ...details.responseHeaders,
        'Content-Security-Policy': [
          "default-src 'self'; " +
          "script-src 'self' 'unsafe-inline' 'unsafe-eval' blob:; " +
          "worker-src 'self' blob:; " +
          "style-src 'self' 'unsafe-inline' https://fonts.googleapis.com; " +
          "font-src 'self' https://fonts.gstatic.com; " +
          "connect-src 'self' https: http: ws: wss:; " +
          "img-src 'self' data: https:;"
        ]
      }
    });
  });
}

// ------------------------------------------------------------------------------
// SSH TUNNEL SUPPORT
// Opens an SSH connection to config.sshHost and a local TCP forwarding server that pipes each
// incoming connection through the SSH session to dstHost:dstPort. DB drivers then connect to
// 127.0.0.1:<localPort> as if it were a normal local TCP endpoint. This only supports the
// discrete host/port connection mode (not a raw connection string), since a connection string
// bakes in the real target host directly.
function openSshTunnel(config, dstHost, dstPort) {
  return new Promise((resolve, reject) => {
    const sshClient = new SshClient();
    let settled = false;

    sshClient.on('ready', () => {
      const localServer = net.createServer((socket) => {
        sshClient.forwardOut(
          socket.remoteAddress || '127.0.0.1',
          socket.remotePort || 0,
          dstHost,
          dstPort,
          (err, stream) => {
            if (err) {
              socket.destroy();
              return;
            }
            socket.pipe(stream);
            stream.pipe(socket);
            stream.on('error', () => socket.destroy());
            socket.on('error', () => stream.destroy());
          }
        );
      });

      localServer.on('error', (err) => {
        if (settled) return;
        settled = true;
        sshClient.end();
        reject(err);
      });

      localServer.listen(0, '127.0.0.1', () => {
        if (settled) return;
        settled = true;
        resolve({ sshClient, localServer, localPort: localServer.address().port });
      });
    });

    sshClient.on('error', (err) => {
      if (settled) return;
      settled = true;
      reject(err);
    });

    sshClient.connect({
      host: config.sshHost,
      port: parseInt(config.sshPort, 10) || 22,
      username: config.sshUsername,
      password: config.sshPassword || undefined,
      privateKey: config.sshPrivateKey || undefined,
      passphrase: config.sshPassphrase || undefined,
      readyTimeout: 10000
    });
  });
}

function closeSshTunnel(tunnel) {
  if (!tunnel) return;
  try { tunnel.localServer.close(); } catch {}
  try { tunnel.sshClient.end(); } catch {}
}

// Resolves the host/port a pooled DB driver should actually connect to: either the real remote
// target, or 127.0.0.1:<localPort> of a persistent tunnel cached alongside that pool for config.id.
async function getEffectiveTarget(config, defaultPort) {
  if (config.sshEnabled && config.sshHost && !config.connectionString) {
    let tunnel = sshTunnels.get(config.id);
    if (!tunnel) {
      const dstHost = config.host || 'localhost';
      const dstPort = parseInt(config.port, 10) || defaultPort;
      tunnel = await openSshTunnel(config, dstHost, dstPort);
      sshTunnels.set(config.id, tunnel);
    }
    return { host: '127.0.0.1', port: tunnel.localPort };
  }
  return { host: config.host || 'localhost', port: parseInt(config.port, 10) || defaultPort };
}

// ------------------------------------------------------------------------------
// NATIVE DATABASE DRIVER IPC HANDLERS (PostgreSQL, MySQL, MongoDB)
// Helper to build robust Postgres config with auto SSL, TLS SNI & custom CA / mTLS certificates
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
      // Always validated against the real remote host, even when connecting via a local tunnel port.
      rejectUnauthorized: config.sslRejectUnauthorized !== undefined ? Boolean(config.sslRejectUnauthorized) : (config.sslCaCert ? true : false),
      servername: host || undefined
    };
    if (config.sslCaCert) {
      sslConfig.ca = config.sslCaCert;
    }
    if (config.sslClientCert) {
      sslConfig.cert = config.sslClientCert;
    }
    if (config.sslClientKey) {
      sslConfig.key = config.sslClientKey;
    }
  }

  // If a full connection string URI is provided, use it directly with SSL SNI options
  if (config.connectionString) {
    return {
      connectionString: config.connectionString,
      ssl: sslConfig,
      connectionTimeoutMillis: 10000
    };
  }

  // Otherwise use discrete credentials parameters
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

// Shared pool/client getters used by both db:query and db:mutate-row so a mutation reuses the exact
// same cached connection a query already opened, rather than a second one racing it.
async function getPgPool(config) {
  let pool = pgPools.get(config.id);
  if (!pool) {
    const pgCfg = config.sshEnabled && config.sshHost && !config.connectionString
      ? getPgConfig(config, await getEffectiveTarget(config, 5432))
      : getPgConfig(config);
    pool = new pg.Pool(pgCfg);
    pgPools.set(config.id, pool);
  }
  return pool;
}

async function getMysqlPool(config) {
  let pool = mysqlPools.get(config.id);
  if (!pool) {
    const sslConfig = getMysqlSslConfig(config);
    if (config.connectionString) {
      pool = mysql.createPool({
        uri: config.connectionString,
        waitForConnections: true,
        connectionLimit: 10,
        connectTimeout: 8000,
        ssl: sslConfig
      });
    } else {
      const target = await getEffectiveTarget(config, 3306);
      pool = mysql.createPool({
        host: target.host,
        port: target.port,
        database: config.database,
        user: config.username || 'root',
        password: config.password || '',
        waitForConnections: true,
        connectionLimit: 10,
        connectTimeout: 8000,
        ssl: sslConfig
      });
    }
    mysqlPools.set(config.id, pool);
  }
  return pool;
}

// Shared SQL builders for a single row mutation, used by both the immediate db:mutate-row handler
// and the transactional db:mutate-batch handler (each statement in a batch still gets its own
// independently-numbered placeholders, since it's executed as its own client.query() call).
function buildPgMutationSql(table, op, values, where) {
  const quoteIdent = (id) => `"${String(id).replace(/"/g, '""')}"`;
  let sql, params;

  if (op === 'insert') {
    const cols = Object.keys(values);
    params = cols.map(c => values[c]);
    sql = `INSERT INTO ${quoteIdent(table)} (${cols.map(quoteIdent).join(', ')}) VALUES (${cols.map((_, i) => `$${i + 1}`).join(', ')})`;
  } else if (op === 'update') {
    const setCols = Object.keys(values);
    params = setCols.map(c => values[c]);
    const setClause = setCols.map((c, i) => `${quoteIdent(c)} = $${i + 1}`).join(', ');
    let paramIdx = setCols.length;
    const whereClause = Object.keys(where).map(c => {
      if (where[c] === null) return `${quoteIdent(c)} IS NULL`;
      paramIdx += 1;
      params.push(where[c]);
      return `${quoteIdent(c)} = $${paramIdx}`;
    }).join(' AND ');
    sql = `UPDATE ${quoteIdent(table)} SET ${setClause} WHERE ${whereClause}`;
  } else if (op === 'delete') {
    params = [];
    let paramIdx = 0;
    const whereClause = Object.keys(where).map(c => {
      if (where[c] === null) return `${quoteIdent(c)} IS NULL`;
      paramIdx += 1;
      params.push(where[c]);
      return `${quoteIdent(c)} = $${paramIdx}`;
    }).join(' AND ');
    sql = `DELETE FROM ${quoteIdent(table)} WHERE ${whereClause}`;
  } else {
    throw new Error(`Unknown row mutation op: ${op}`);
  }
  return { sql, params };
}

function buildMysqlMutationSql(table, op, values, where) {
  const quoteIdent = (id) => `\`${String(id).replace(/`/g, '``')}\``;
  let sql, params;

  if (op === 'insert') {
    const cols = Object.keys(values);
    params = cols.map(c => values[c]);
    sql = `INSERT INTO ${quoteIdent(table)} (${cols.map(quoteIdent).join(', ')}) VALUES (${cols.map(() => '?').join(', ')})`;
  } else if (op === 'update') {
    const setCols = Object.keys(values);
    const whereCols = Object.keys(where);
    params = [...setCols.map(c => values[c]), ...whereCols.filter(c => where[c] !== null).map(c => where[c])];
    const setClause = setCols.map(c => `${quoteIdent(c)} = ?`).join(', ');
    const whereClause = whereCols.map(c => where[c] === null ? `${quoteIdent(c)} IS NULL` : `${quoteIdent(c)} = ?`).join(' AND ');
    sql = `UPDATE ${quoteIdent(table)} SET ${setClause} WHERE ${whereClause}`;
  } else if (op === 'delete') {
    const whereCols = Object.keys(where);
    params = whereCols.filter(c => where[c] !== null).map(c => where[c]);
    const whereClause = whereCols.map(c => where[c] === null ? `${quoteIdent(c)} IS NULL` : `${quoteIdent(c)} = ?`).join(' AND ');
    sql = `DELETE FROM ${quoteIdent(table)} WHERE ${whereClause}`;
  } else {
    throw new Error(`Unknown row mutation op: ${op}`);
  }
  return { sql, params };
}

async function getMongoDb(config) {
  let client = mongoClients.get(config.id);
  if (!client) {
    let uri = config.connectionString;
    if (!uri) {
      const auth = config.username && config.password ? `${encodeURIComponent(config.username)}:${encodeURIComponent(config.password)}@` : '';
      const target = await getEffectiveTarget(config, 27017);
      uri = `mongodb://${auth}${target.host}:${target.port}/${config.database || 'test'}`;
    }
    client = new MongoClient(uri);
    await client.connect();
    mongoClients.set(config.id, client);
  }
  return client.db(config.database || 'test');
}

// Builds a lazily-connecting ioredis client. `connectOverride` (used for SSH-tunneled connections)
// swaps in the local tunnel host/port while a raw connection string (which bakes in its own host)
// is left untouched — tunneling isn't supported in connection-string mode, same as the other drivers.
const REDIS_COMMON_OPTS = {
  connectTimeout: 8000,
  lazyConnect: true,
  maxRetriesPerRequest: 1,
  retryStrategy: (times) => (times > 2 ? null : 200)
};

function buildRedisClient(config, connectOverride) {
  let client;
  if (config.connectionString && !connectOverride) {
    client = new Redis(config.connectionString, REDIS_COMMON_OPTS);
  } else {
    // Redis has no valid auth combination for "username with no password" — a lone username
    // (e.g. a stale default left over from switching driver types in the UI) would otherwise
    // make ioredis attempt to authenticate and get rejected by servers that don't expect it.
    const opts = {
      ...REDIS_COMMON_OPTS,
      host: (connectOverride ? connectOverride.host : config.host) || 'localhost',
      port: connectOverride ? connectOverride.port : (parseInt(config.port, 10) || 6379),
      username: config.password ? (config.username || undefined) : undefined,
      password: config.password || undefined,
      db: parseInt(config.database, 10) || 0
    };

    if (config.ssl) {
      opts.tls = {
        rejectUnauthorized: config.sslRejectUnauthorized !== undefined ? Boolean(config.sslRejectUnauthorized) : true,
        ca: config.sslCaCert || undefined,
        cert: config.sslClientCert || undefined,
        key: config.sslClientKey || undefined
      };
    }

    client = new Redis(opts);
  }
  // We always await connect()/commands and handle rejection ourselves — this just stops ioredis's
  // EventEmitter from logging an "unhandled error" warning on top of that.
  client.on('error', () => {});
  return client;
}

// Splits a Redis command line into tokens, respecting single/double-quoted arguments
// (e.g. SET mykey "hello world").
function parseRedisCommandLine(line) {
  const tokens = [];
  const regex = /"([^"]*)"|'([^']*)'|(\S+)/g;
  let match;
  while ((match = regex.exec(line)) !== null) {
    tokens.push(match[1] !== undefined ? match[1] : match[2] !== undefined ? match[2] : match[3]);
  }
  return tokens;
}

// Normalizes Redis's heterogeneous reply shapes into the app's uniform columns/rows grid.
function formatRedisResult(result) {
  if (result === null || result === undefined) {
    return { columns: ['result'], rows: [{ result: '(nil)' }] };
  }
  if (Array.isArray(result)) {
    return {
      columns: ['index', 'value'],
      rows: result.map((v, i) => ({
        index: i,
        value: v === null || v === undefined ? '(nil)' : (typeof v === 'object' ? JSON.stringify(v) : String(v))
      }))
    };
  }
  if (typeof result === 'object') {
    return {
      columns: ['field', 'value'],
      rows: Object.entries(result).map(([field, value]) => ({ field, value: String(value) }))
    };
  }
  return { columns: ['result'], rows: [{ result: String(result) }] };
}

// 1. Test Database Connection
ipcMain.handle('db:test-connection', async (event, config) => {
  const startTime = Date.now();
  const useTunnel = Boolean(config.sshEnabled && config.sshHost && !config.connectionString);
  let tunnel = null;
  try {
    if (config.type === 'postgres') {
      let pgCfg;
      if (useTunnel) {
        tunnel = await openSshTunnel(config, config.host || 'localhost', parseInt(config.port, 10) || 5432);
        pgCfg = getPgConfig(config, { host: '127.0.0.1', port: tunnel.localPort });
      } else {
        pgCfg = getPgConfig(config);
      }
      const client = new pg.Client(pgCfg);
      await client.connect();
      await client.query('SELECT 1');
      await client.end();
      const latencyMs = Date.now() - startTime;
      return { success: true, latencyMs, message: `Connected to PostgreSQL successfully (${latencyMs}ms)${useTunnel ? ' via SSH tunnel' : ''}` };
    }

    if (config.type === 'mysql') {
      const sslConfig = getMysqlSslConfig(config);
      let connection;
      if (config.connectionString) {
        connection = await mysql.createConnection({
          uri: config.connectionString,
          connectTimeout: 8000,
          ssl: sslConfig
        });
      } else {
        let targetHost = config.host || 'localhost';
        let targetPort = parseInt(config.port, 10) || 3306;
        if (useTunnel) {
          tunnel = await openSshTunnel(config, targetHost, targetPort);
          targetHost = '127.0.0.1';
          targetPort = tunnel.localPort;
        }
        connection = await mysql.createConnection({
          host: targetHost,
          port: targetPort,
          database: config.database,
          user: config.username || 'root',
          password: config.password || '',
          connectTimeout: 8000,
          ssl: sslConfig
        });
      }
      await connection.query('SELECT 1');
      await connection.end();
      const latencyMs = Date.now() - startTime;
      return { success: true, latencyMs, message: `Connected to MySQL successfully (${latencyMs}ms)${useTunnel ? ' via SSH tunnel' : ''}` };
    }

    if (config.type === 'mongodb') {
      let uri = config.connectionString;
      if (!uri) {
        const auth = config.username && config.password ? `${encodeURIComponent(config.username)}:${encodeURIComponent(config.password)}@` : '';
        let host = config.host || 'localhost';
        let port = config.port || 27017;
        if (useTunnel) {
          tunnel = await openSshTunnel(config, host, parseInt(port, 10) || 27017);
          host = '127.0.0.1';
          port = tunnel.localPort;
        }
        uri = `mongodb://${auth}${host}:${port}/${config.database || 'admin'}`;
      }
      const client = new MongoClient(uri, { serverSelectionTimeoutMS: 7000 });
      await client.connect();
      await client.db(config.database || 'admin').command({ ping: 1 });
      await client.close();
      const latencyMs = Date.now() - startTime;
      return { success: true, latencyMs, message: `Connected to MongoDB successfully (${latencyMs}ms)${useTunnel ? ' via SSH tunnel' : ''}` };
    }

    if (config.type === 'redis') {
      let redisTarget = null;
      if (useTunnel) {
        tunnel = await openSshTunnel(config, config.host || 'localhost', parseInt(config.port, 10) || 6379);
        redisTarget = { host: '127.0.0.1', port: tunnel.localPort };
      }
      const client = buildRedisClient(config, redisTarget);
      try {
        await client.connect();
        await client.ping();
      } finally {
        client.disconnect();
      }
      const latencyMs = Date.now() - startTime;
      return { success: true, latencyMs, message: `Connected to Redis successfully (${latencyMs}ms)${useTunnel ? ' via SSH tunnel' : ''}` };
    }

    return { success: true, latencyMs: 5, message: `Driver ready` };
  } catch (err) {
    let friendlyMessage = err.message || 'Database connection failed';
    if (useTunnel && (err.level === 'client-authentication' || err.level === 'client-timeout' || err.message?.includes(config.sshHost))) {
      friendlyMessage = `SSH tunnel error connecting to ${config.sshHost}:${config.sshPort || 22} as "${config.sshUsername || ''}": ${err.message}`;
    } else if (err.code === 'ECONNREFUSED' || err.message?.includes('ECONNREFUSED')) {
      const defaultPort = DEFAULT_DB_PORTS[config.type] || 5432;
      friendlyMessage = `Could not connect to ${config.type?.toUpperCase() || 'Database'} on ${config.host || 'localhost'}:${config.port || defaultPort} (Connection Refused). Verify host address and port.`;
    } else if (err.message?.includes('password authentication failed')) {
      friendlyMessage = `Authentication failed: Incorrect username or password for user "${config.username || 'postgres'}".`;
    } else if (err.message?.includes('WRONGPASS') || err.message?.includes('NOAUTH') || err.message?.includes('NOPERM')) {
      friendlyMessage = `Redis authentication failed: ${err.message}. Verify the username/password (or ACL permissions) for this connection.`;
    } else if (err.code === 'ETIMEDOUT' || err.message?.includes('timeout')) {
      friendlyMessage = `Connection timed out reaching ${config.host || 'server'}:${config.port || 'port'}. Verify server address, port, and firewall rules.`;
    }
    return { success: false, message: friendlyMessage };
  } finally {
    closeSshTunnel(tunnel);
  }
});

// 2. Execute SQL / MongoDB Query
ipcMain.handle('db:query', async (event, { config, sql }) => {
  const startTime = Date.now();
  try {
    if (config.type === 'postgres') {
      let pool = pgPools.get(config.id);
      if (!pool) {
        const pgCfg = config.sshEnabled && config.sshHost && !config.connectionString
          ? getPgConfig(config, await getEffectiveTarget(config, 5432))
          : getPgConfig(config);
        pool = new pg.Pool(pgCfg);
        pgPools.set(config.id, pool);
      }

      const res = await pool.query(sql);
      const executionTimeMs = Date.now() - startTime;
      const columns = res.fields ? res.fields.map(f => f.name) : (res.rows[0] ? Object.keys(res.rows[0]) : []);

      let queryPlan = undefined;
      if (columns.length === 1 && (columns[0].toLowerCase().includes('plan') || columns[0].toLowerCase().includes('explain'))) {
        queryPlan = res.rows.map(r => Object.values(r)[0]).join('\n');
      }

      return {
        success: true,
        columns,
        rows: res.rows || [],
        rowCount: res.rowCount ?? (res.rows ? res.rows.length : 0),
        executionTimeMs,
        command: res.command || undefined,
        queryPlan
      };
    }

    if (config.type === 'mysql') {
      let pool = mysqlPools.get(config.id);
      if (!pool) {
        const sslConfig = getMysqlSslConfig(config);
        if (config.connectionString) {
          pool = mysql.createPool({
            uri: config.connectionString,
            waitForConnections: true,
            connectionLimit: 10,
            connectTimeout: 8000,
            ssl: sslConfig
          });
        } else {
          const target = await getEffectiveTarget(config, 3306);
          pool = mysql.createPool({
            host: target.host,
            port: target.port,
            database: config.database,
            user: config.username || 'root',
            password: config.password || '',
            waitForConnections: true,
            connectionLimit: 10,
            connectTimeout: 8000,
            ssl: sslConfig
          });
        }
        mysqlPools.set(config.id, pool);
      }

      const [rows, fields] = await pool.query(sql);
      const executionTimeMs = Date.now() - startTime;
      const columns = fields ? fields.map(f => f.name) : (Array.isArray(rows) && rows[0] ? Object.keys(rows[0]) : []);

      let queryPlan = undefined;
      if (sql.trim().toUpperCase().startsWith('EXPLAIN')) {
        queryPlan = JSON.stringify(rows, null, 2);
      }

      return {
        success: true,
        columns,
        rows: Array.isArray(rows) ? rows : [],
        rowCount: Array.isArray(rows) ? rows.length : (rows.affectedRows || 0),
        executionTimeMs,
        command: sql.trim().split(/\s+/)[0]?.toUpperCase(),
        queryPlan
      };
    }

    if (config.type === 'mongodb') {
      let client = mongoClients.get(config.id);
      if (!client) {
        let uri = config.connectionString;
        if (!uri) {
          const auth = config.username && config.password ? `${encodeURIComponent(config.username)}:${encodeURIComponent(config.password)}@` : '';
          const target = await getEffectiveTarget(config, 27017);
          uri = `mongodb://${auth}${target.host}:${target.port}/${config.database || 'test'}`;
        }
        client = new MongoClient(uri);
        await client.connect();
        mongoClients.set(config.id, client);
      }

      const db = client.db(config.database || 'test');

      // Parse query string (supports JSON filter or collection.find format)
      let collectionName = config.tables?.[0]?.name || 'documents';
      let filter = {};

      const trimmed = sql.trim();
      if (trimmed.startsWith('{') && trimmed.endsWith('}')) {
        try { filter = JSON.parse(trimmed); } catch {}
      } else {
        // Match e.g. "users.find({ status: 'active' })"
        const match = trimmed.match(/^([a-zA-Z0-9_-]+)\.find\((.*)\)/);
        if (match) {
          collectionName = match[1];
          try { filter = JSON.parse(match[2] || '{}'); } catch {}
        }
      }

      const collection = db.collection(collectionName);
      const docs = await collection.find(filter).limit(50).toArray();
      const executionTimeMs = Date.now() - startTime;

      // Extract unique keys across documents for table columns
      const keySet = new Set();
      docs.forEach(doc => Object.keys(doc).forEach(k => keySet.add(k)));
      const columns = Array.from(keySet);

      // Serialize ObjectIds for UI display
      const rows = docs.map(d => {
        const clean = { ...d };
        if (clean._id) clean._id = String(clean._id);
        return clean;
      });

      return {
        success: true,
        columns,
        rows,
        rowCount: rows.length,
        executionTimeMs
      };
    }

    if (config.type === 'redis') {
      let client = redisClients.get(config.id);
      if (!client) {
        const target = config.sshEnabled && config.sshHost && !config.connectionString
          ? await getEffectiveTarget(config, 6379)
          : null;
        client = buildRedisClient(config, target);
        await client.connect();
        redisClients.set(config.id, client);
      }

      const tokens = parseRedisCommandLine(sql.trim());
      if (tokens.length === 0) {
        return { success: false, message: 'Enter a Redis command, e.g. GET mykey or KEYS *', executionTimeMs: Date.now() - startTime };
      }

      const [cmd, ...args] = tokens;
      const raw = await client.call(cmd, ...args);
      const { columns, rows } = formatRedisResult(raw);
      const executionTimeMs = Date.now() - startTime;

      return {
        success: true,
        columns,
        rows,
        rowCount: rows.length,
        executionTimeMs,
        command: cmd.toUpperCase()
      };
    }

    return { success: false, message: `Unsupported driver: ${config.type}` };
  } catch (err) {
    let friendlyMessage = err.message || 'Query execution error';
    if (err.code === 'ECONNREFUSED' || err.message?.includes('ECONNREFUSED')) {
      const defaultPort = DEFAULT_DB_PORTS[config.type] || 5432;
      friendlyMessage = `Could not connect to ${config.type?.toUpperCase() || 'Database'} on ${config.host || 'localhost'}:${config.port || defaultPort} (Connection Refused). No database service is running on this port.`;
    } else if (err.message?.includes('password authentication failed')) {
      friendlyMessage = `Authentication failed: Incorrect username or password for user "${config.username || 'postgres'}".`;
    } else if (err.message?.includes('WRONGPASS') || err.message?.includes('NOAUTH') || err.message?.includes('NOPERM')) {
      friendlyMessage = `Redis authentication failed: ${err.message}. Verify the username/password (or ACL permissions) for this connection.`;
    }
    return {
      success: false,
      message: friendlyMessage,
      executionTimeMs: Date.now() - startTime
    };
  }
});

// 3. Mutate several rows/documents as one all-or-nothing batch — backs the Data Grid's DBeaver-style
// "pending changes, then Save/Revert" model. Real transactions (BEGIN/COMMIT/ROLLBACK, or a Mongo
// session) for Postgres/MySQL/MongoDB *when the server supports them*. A standalone (non-replica-set)
// MongoDB deployment has no multi-document transaction support at all — rather than silently applying
// changes one-by-one and calling that "saved" the same way, this falls back to sequential execution
// but reports `atomic: false` and stops immediately on the first failure, so the renderer can tell the
// user exactly what happened instead of implying an all-or-nothing guarantee that wasn't real.
ipcMain.handle('db:mutate-batch', async (event, { config, table, mutations }) => {
  const startTime = Date.now();
  try {
    if (!Array.isArray(mutations) || mutations.length === 0) {
      return { success: true, atomic: true, results: [], executionTimeMs: Date.now() - startTime };
    }

    if (config.type === 'postgres') {
      const pool = await getPgPool(config);
      const client = await pool.connect();
      try {
        await client.query('BEGIN');
        const results = [];
        for (const m of mutations) {
          const { sql, params } = buildPgMutationSql(table, m.op, m.values, m.where);
          const res = await client.query(sql, params);
          results.push({ op: m.op, rowsAffected: res.rowCount ?? 0 });
        }
        await client.query('COMMIT');
        return { success: true, atomic: true, results, executionTimeMs: Date.now() - startTime };
      } catch (err) {
        await client.query('ROLLBACK').catch(() => {});
        throw err;
      } finally {
        client.release();
      }
    }

    if (config.type === 'mysql') {
      const pool = await getMysqlPool(config);
      const conn = await pool.getConnection();
      try {
        await conn.beginTransaction();
        const results = [];
        for (const m of mutations) {
          const { sql, params } = buildMysqlMutationSql(table, m.op, m.values, m.where);
          const [res] = await conn.query(sql, params);
          results.push({ op: m.op, rowsAffected: res.affectedRows ?? 0 });
        }
        await conn.commit();
        return { success: true, atomic: true, results, executionTimeMs: Date.now() - startTime };
      } catch (err) {
        await conn.rollback().catch(() => {});
        throw err;
      } finally {
        conn.release();
      }
    }

    if (config.type === 'mongodb') {
      const db = await getMongoDb(config);
      const collection = db.collection(table);
      const toObjectIdIfValid = (v) => (typeof v === 'string' && ObjectId.isValid(v) && String(new ObjectId(v)) === v) ? new ObjectId(v) : v;

      const applyOne = async (m, session) => {
        const opts = session ? { session } : {};
        if (m.op === 'insert') {
          const doc = { ...m.values };
          delete doc._id;
          const res = await collection.insertOne(doc, opts);
          return { op: 'insert', rowsAffected: 1, insertedId: String(res.insertedId) };
        }
        if (!m.where || m.where._id === undefined) {
          throw new Error('Missing _id to target a MongoDB document');
        }
        const filter = { _id: toObjectIdIfValid(m.where._id) };
        if (m.op === 'update') {
          const setDoc = { ...m.values };
          delete setDoc._id;
          const res = await collection.updateOne(filter, { $set: setDoc }, opts);
          return { op: 'update', rowsAffected: res.modifiedCount ?? 0 };
        }
        if (m.op === 'delete') {
          const res = await collection.deleteOne(filter, opts);
          return { op: 'delete', rowsAffected: res.deletedCount ?? 0 };
        }
        throw new Error(`Unknown row mutation op: ${m.op}`);
      };

      const client = mongoClients.get(config.id);
      let session = null;
      try {
        session = client.startSession();
        const results = [];
        await session.withTransaction(async () => {
          for (const m of mutations) {
            results.push(await applyOne(m, session));
          }
        });
        return { success: true, atomic: true, results, executionTimeMs: Date.now() - startTime };
      } catch (err) {
        const noTransactionSupport = /Transaction numbers are only allowed|IllegalOperation|Sessions are not supported|not supported.*replica set/i.test(err.message || '');
        if (!noTransactionSupport) throw err;

        // Standalone MongoDB: no transaction support at all. Apply sequentially and stop at the
        // first failure — already-applied changes before it stay applied and are NOT rolled back,
        // which the response says explicitly rather than pretending otherwise.
        const results = [];
        for (let i = 0; i < mutations.length; i++) {
          try {
            results.push(await applyOne(mutations[i], null));
          } catch (innerErr) {
            return {
              success: false,
              atomic: false,
              message: `Change ${i + 1} of ${mutations.length} failed (${innerErr.message}). This MongoDB server doesn't support multi-document transactions (not a replica set), so the ${i} change(s) before it were already applied and were NOT rolled back.`,
              results,
              executionTimeMs: Date.now() - startTime
            };
          }
        }
        return {
          success: true,
          atomic: false,
          message: 'Applied sequentially, not atomically — this MongoDB server is not a replica set, so multi-document transactions are unavailable.',
          results,
          executionTimeMs: Date.now() - startTime
        };
      } finally {
        if (session) await session.endSession().catch(() => {});
      }
    }

    return { success: false, message: `Batch row editing is not supported for the ${config.type} driver.` };
  } catch (err) {
    return {
      success: false,
      message: err.message || 'Batch mutation failed',
      executionTimeMs: Date.now() - startTime
    };
  }
});

// 4. Fetch Real Database Schema Tables & Collections
ipcMain.handle('db:get-schema', async (event, config) => {
  try {
    if (config.type === 'postgres') {
      let pool = pgPools.get(config.id);
      if (!pool) {
        const pgCfg = config.sshEnabled && config.sshHost && !config.connectionString
          ? getPgConfig(config, await getEffectiveTarget(config, 5432))
          : getPgConfig(config);
        pool = new pg.Pool(pgCfg);
        pgPools.set(config.id, pool);
      }

      let tableRes;
      try {
        tableRes = await pool.query(`
          SELECT table_name 
          FROM information_schema.tables 
          WHERE table_schema = 'public' 
            AND table_type = 'BASE TABLE'
            AND table_name NOT LIKE 'pg_%'
            AND table_name NOT LIKE 'sql_%'
          ORDER BY table_name
          LIMIT 50;
        `);
      } catch {
        tableRes = { rows: [] };
      }

      if (!tableRes.rows || tableRes.rows.length === 0) {
        try {
          tableRes = await pool.query(`
            SELECT table_name 
            FROM information_schema.tables 
            WHERE table_schema = 'public' 
            ORDER BY table_name
            LIMIT 40;
          `);
        } catch {
          tableRes = { rows: [] };
        }
      }

      const tables = [];
      for (const row of (tableRes.rows || [])) {
        const tableName = row.table_name;
        try {
          const colRes = await pool.query(`
            SELECT column_name, data_type, is_nullable
            FROM information_schema.columns
            WHERE table_name = $1 AND table_schema = 'public';
          `, [tableName]);

          tables.push({
            name: tableName,
            rowCount: 500,
            columns: (colRes.rows || []).map(c => ({
              name: c.column_name,
              type: (c.data_type || 'VARCHAR').toUpperCase(),
              isPrimaryKey: c.column_name === 'id',
              isNullable: c.is_nullable === 'YES'
            }))
          });
        } catch {
          tables.push({
            name: tableName,
            rowCount: 0,
            columns: [{ name: 'id', type: 'INT', isPrimaryKey: true, isNullable: false }]
          });
        }
      }

      return { success: true, tables };
    }

    if (config.type === 'mysql') {
      let pool = mysqlPools.get(config.id);
      if (!pool) {
        const sslConfig = getMysqlSslConfig(config);
        if (config.connectionString) {
          pool = mysql.createPool({
            uri: config.connectionString,
            waitForConnections: true,
            connectionLimit: 10,
            connectTimeout: 8000,
            ssl: sslConfig
          });
        } else {
          const target = await getEffectiveTarget(config, 3306);
          pool = mysql.createPool({
            host: target.host,
            port: target.port,
            database: config.database,
            user: config.username || 'root',
            password: config.password || '',
            waitForConnections: true,
            connectionLimit: 10,
            connectTimeout: 8000,
            ssl: sslConfig
          });
        }
        mysqlPools.set(config.id, pool);
      }

      const dbName = config.database || 'hg38';
      let tableNames = [];

      try {
        const [tableRows] = await pool.query(
          'SELECT table_name FROM information_schema.tables WHERE table_schema = ? ORDER BY table_name LIMIT 30;',
          [dbName]
        );
        tableNames = (tableRows || []).map(r => r.TABLE_NAME || r.table_name).filter(Boolean);
      } catch {
        try {
          const [rawRows] = await pool.query('SHOW TABLES');
          tableNames = (rawRows || []).slice(0, 30).map(r => Object.values(r)[0]).filter(Boolean);
        } catch {}
      }

      const tables = [];
      for (const tableName of tableNames) {
        try {
          const safeName = String(tableName).replace(/`/g, '``');
          const [colRows] = await pool.query(`DESCRIBE \`${safeName}\``);
          tables.push({
            name: tableName,
            rowCount: 1000,
            columns: (colRows || []).map(c => ({
              name: c.Field,
              type: (c.Type || 'VARCHAR').toUpperCase(),
              isPrimaryKey: c.Key === 'PRI',
              isNullable: c.Null === 'YES'
            }))
          });
        } catch {
          tables.push({
            name: tableName,
            rowCount: 0,
            columns: [{ name: 'id', type: 'INT', isPrimaryKey: true, isNullable: false }]
          });
        }
      }

      return { success: true, tables };
    }

    if (config.type === 'mongodb') {
      let client = mongoClients.get(config.id);
      if (!client) {
        let uri = config.connectionString;
        if (!uri) {
          const auth = config.username && config.password ? `${encodeURIComponent(config.username)}:${encodeURIComponent(config.password)}@` : '';
          const target = await getEffectiveTarget(config, 27017);
          uri = `mongodb://${auth}${target.host}:${target.port}/${config.database || 'test'}`;
        }
        client = new MongoClient(uri);
        await client.connect();
        mongoClients.set(config.id, client);
      }

      const db = client.db(config.database || 'test');
      const collections = await db.listCollections().toArray();
      const tables = [];

      for (const col of collections) {
        const count = await db.collection(col.name).countDocuments();
        const sampleDoc = await db.collection(col.name).findOne({});
        const columns = sampleDoc ? Object.keys(sampleDoc).map(k => ({
          name: k,
          type: typeof sampleDoc[k] === 'object' ? 'OBJECT' : typeof sampleDoc[k].toUpperCase?.() || 'STRING',
          isPrimaryKey: k === '_id',
          isNullable: true
        })) : [{ name: '_id', type: 'OBJECTID', isPrimaryKey: true, isNullable: false }];

        tables.push({
          name: col.name,
          rowCount: count,
          columns
        });
      }

      return { success: true, tables };
    }

    if (config.type === 'redis') {
      let client = redisClients.get(config.id);
      if (!client) {
        const target = config.sshEnabled && config.sshHost && !config.connectionString
          ? await getEffectiveTarget(config, 6379)
          : null;
        client = buildRedisClient(config, target);
        await client.connect();
        redisClients.set(config.id, client);
      }

      // Redis has no tables — group a sample of the keyspace by Redis TYPE instead, so the
      // schema browser shows something meaningful (e.g. "string", "hash", "list" pseudo-tables).
      const typeGroups = {};
      let cursor = '0';
      let scanned = 0;
      do {
        const [nextCursor, keys] = await client.call('SCAN', cursor, 'COUNT', '200');
        cursor = nextCursor;
        if (keys.length > 0) {
          const pipeline = client.pipeline();
          keys.forEach(k => pipeline.call('TYPE', k));
          const typeResults = await pipeline.exec();
          keys.forEach((key, idx) => {
            const type = (typeResults[idx] && !typeResults[idx][0] && typeResults[idx][1]) || 'unknown';
            if (!typeGroups[type]) typeGroups[type] = [];
            if (typeGroups[type].length < 50) typeGroups[type].push(key);
          });
        }
        scanned += keys.length;
      } while (cursor !== '0' && scanned < 1000);

      const tables = Object.entries(typeGroups).map(([type, keys]) => ({
        name: type,
        rowCount: keys.length,
        columns: keys.map(k => ({ name: k, type: 'KEY', isPrimaryKey: false, isNullable: true }))
      }));

      return { success: true, tables };
    }

    return { success: false, tables: [] };
  } catch (err) {
    return { success: false, message: err.message, tables: [] };
  }
});

// OS-keychain-backed encryption for secrets persisted by the renderer (DB passwords, API keys, etc).
// Ciphertext is prefixed so plaintext written by older versions still round-trips untouched.
const SECURE_PREFIX = 'enc1:';

ipcMain.on('secure:encrypt-sync', (event, plainText) => {
  try {
    if (typeof plainText !== 'string' || !safeStorage.isEncryptionAvailable()) {
      event.returnValue = plainText;
      return;
    }
    const encrypted = safeStorage.encryptString(plainText);
    event.returnValue = SECURE_PREFIX + encrypted.toString('base64');
  } catch {
    event.returnValue = plainText;
  }
});

ipcMain.on('secure:decrypt-sync', (event, storedText) => {
  try {
    if (typeof storedText !== 'string' || !storedText.startsWith(SECURE_PREFIX)) {
      event.returnValue = storedText;
      return;
    }
    if (!safeStorage.isEncryptionAvailable()) {
      event.returnValue = '';
      return;
    }
    const buffer = Buffer.from(storedText.slice(SECURE_PREFIX.length), 'base64');
    event.returnValue = safeStorage.decryptString(buffer);
  } catch {
    event.returnValue = '';
  }
});

// 5. Disconnect / invalidate a cached pool or client (called when a connection is edited or removed,
// since edits reuse the same config.id and would otherwise keep querying through stale credentials)
ipcMain.handle('db:disconnect', async (event, id) => {
  try {
    const pgPool = pgPools.get(id);
    if (pgPool) {
      pgPools.delete(id);
      await pgPool.end().catch(() => {});
    }
    const mysqlPool = mysqlPools.get(id);
    if (mysqlPool) {
      mysqlPools.delete(id);
      await mysqlPool.end().catch(() => {});
    }
    const mongoClient = mongoClients.get(id);
    if (mongoClient) {
      mongoClients.delete(id);
      await mongoClient.close().catch(() => {});
    }
    const redisClient = redisClients.get(id);
    if (redisClient) {
      redisClients.delete(id);
      redisClient.disconnect();
    }
    const tunnel = sshTunnels.get(id);
    if (tunnel) {
      sshTunnels.delete(id);
      closeSshTunnel(tunnel);
    }
    return { success: true };
  } catch (err) {
    return { success: false, message: err.message };
  }
});

// ------------------------------------------------------------------------------
// GIT-FRIENDLY COLLECTION FOLDER SYNC
// Mirrors a collection's requests to plain JSON files on disk (one file per request, plus a
// small manifest) so they can be committed to git and reviewed/diffed like code, Bruno-style.
function safeFileId(id) {
  return String(id).replace(/[^a-zA-Z0-9_.-]/g, '_');
}

ipcMain.handle('fs:choose-folder', async () => {
  const win = BrowserWindow.getFocusedWindow();
  const result = await dialog.showOpenDialog(win, {
    properties: ['openDirectory', 'createDirectory'],
    title: 'Choose a folder for this collection'
  });
  if (result.canceled || !result.filePaths[0]) return { success: false };
  return { success: true, folderPath: result.filePaths[0] };
});

ipcMain.handle('fs:write-collection-folder', async (event, { folderPath, collectionName, requests }) => {
  try {
    const requestsDir = path.join(folderPath, 'requests');
    await fs.mkdir(requestsDir, { recursive: true });

    const requestOrder = requests.map(r => r.id);
    const manifest = { name: collectionName, requestOrder };
    await fs.writeFile(path.join(folderPath, 'collection.json'), JSON.stringify(manifest, null, 2) + '\n', 'utf8');

    const keepFiles = new Set(requestOrder.map(id => safeFileId(id) + '.json'));
    for (const req of requests) {
      const fileName = safeFileId(req.id) + '.json';
      await fs.writeFile(path.join(requestsDir, fileName), JSON.stringify(req, null, 2) + '\n', 'utf8');
    }

    // Prune files for requests that were removed from the collection, so deletions are reflected on disk too.
    let existingFiles = [];
    try {
      existingFiles = await fs.readdir(requestsDir);
    } catch {}
    for (const file of existingFiles) {
      if (file.endsWith('.json') && !keepFiles.has(file)) {
        await fs.unlink(path.join(requestsDir, file)).catch(() => {});
      }
    }

    return { success: true };
  } catch (err) {
    return { success: false, message: err.message };
  }
});

ipcMain.handle('fs:read-collection-folder', async (event, folderPath) => {
  try {
    const manifestRaw = await fs.readFile(path.join(folderPath, 'collection.json'), 'utf8');
    const manifest = JSON.parse(manifestRaw);
    const requestsDir = path.join(folderPath, 'requests');

    const requests = [];
    for (const id of (manifest.requestOrder || [])) {
      try {
        const raw = await fs.readFile(path.join(requestsDir, safeFileId(id) + '.json'), 'utf8');
        requests.push(JSON.parse(raw));
      } catch {}
    }

    return { success: true, name: manifest.name || path.basename(folderPath), requests };
  } catch (err) {
    return { success: false, message: `Could not read a Bapu collection from this folder: ${err.message}` };
  }
});

// Open external URLs in default system browser
ipcMain.handle('app:open-external', async (event, url) => {
  if (typeof url === 'string' && (url.startsWith('https:') || url.startsWith('http:'))) {
    await shell.openExternal(url);
    return { success: true };
  }
  return { success: false, message: 'Invalid URL scheme' };
});

// App Lifecycle
app.whenReady().then(() => {
  createWindow();

  app.on('activate', () => {
    if (BrowserWindow.getAllWindows().length === 0) createWindow();
  });
});

app.on('window-all-closed', () => {
  // Close all open database pools & clients on shutdown
  for (const pool of pgPools.values()) {
    try { pool.end(); } catch {}
  }
  for (const pool of mysqlPools.values()) {
    try { pool.end(); } catch {}
  }
  for (const client of mongoClients.values()) {
    try { client.close(); } catch {}
  }
  for (const client of redisClients.values()) {
    try { client.disconnect(); } catch {}
  }
  for (const tunnel of sshTunnels.values()) {
    closeSshTunnel(tunnel);
  }
  if (process.platform !== 'darwin') app.quit();
});
