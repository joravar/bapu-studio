import React, { useState } from 'react';
import { X, Database, Check, Zap, Server, Shield, HardDrive, Link2, Sliders, Sparkles } from 'lucide-react';
import { DatabaseConnection } from '../../types';
import { DatabaseService } from '../../services/databaseService';

interface NewConnectionModalProps {
  isOpen: boolean;
  onClose: () => void;
  onConnect: (connection: DatabaseConnection) => void;
}

type DbType = 'postgres' | 'mysql' | 'mongodb' | 'sqlite' | 'redis';
type ConnectionMode = 'uri' | 'params';

export function parseDatabaseUri(uri: string): {
  type?: DbType;
  host?: string;
  port?: string;
  database?: string;
  username?: string;
  password?: string;
  displayName?: string;
} {
  const trimmed = uri.trim();
  if (!trimmed) return {};

  let type: DbType = 'postgres';
  let cleanUri = trimmed;

  if (trimmed.startsWith('postgres://') || trimmed.startsWith('postgresql://')) {
    type = 'postgres';
    cleanUri = trimmed.replace(/^postgres(?:ql)?:\/\//i, '');
  } else if (trimmed.startsWith('mysql://') || trimmed.startsWith('mariadb://')) {
    type = 'mysql';
    cleanUri = trimmed.replace(/^(?:mysql|mariadb):\/\//i, '');
  } else if (trimmed.startsWith('mongodb://') || trimmed.startsWith('mongodb+srv://')) {
    type = 'mongodb';
    cleanUri = trimmed.replace(/^mongodb(?:\+srv)?:\/\//i, '');
  } else if (trimmed.startsWith('redis://') || trimmed.startsWith('rediss://')) {
    type = 'redis';
    cleanUri = trimmed.replace(/^redis(?:s)?:\/\//i, '');
  } else if (trimmed.startsWith('sqlite://') || trimmed.endsWith('.db') || trimmed.endsWith('.sqlite') || trimmed.endsWith('.sqlite3')) {
    type = 'sqlite';
    const dbName = trimmed.replace(/^sqlite:\/\//i, '').replace(/^[\\/]+/, '');
    return { type: 'sqlite', database: dbName, displayName: `SQLite (${dbName || 'Local'})` };
  }

  try {
    let username: string | undefined;
    let password: string | undefined;
    let host: string | undefined;
    let port: string | undefined;
    let database: string | undefined;

    // Check for user:password@host structure
    const lastAtIndex = cleanUri.lastIndexOf('@');
    if (lastAtIndex !== -1) {
      const authPart = cleanUri.substring(0, lastAtIndex);
      const colonIndex = authPart.indexOf(':');
      if (colonIndex !== -1) {
        username = decodeURIComponent(authPart.substring(0, colonIndex));
        password = decodeURIComponent(authPart.substring(colonIndex + 1));
      } else {
        username = decodeURIComponent(authPart);
      }
      cleanUri = cleanUri.substring(lastAtIndex + 1);
    }

    // Parse host, port, database from cleanUri (e.g. host:5432/dbname?sslmode=require)
    const slashIndex = cleanUri.indexOf('/');
    const hostPortPart = slashIndex !== -1 ? cleanUri.substring(0, slashIndex) : cleanUri;
    const pathAndQuery = slashIndex !== -1 ? cleanUri.substring(slashIndex + 1) : '';

    const colonHostIndex = hostPortPart.indexOf(':');
    if (colonHostIndex !== -1) {
      host = hostPortPart.substring(0, colonHostIndex);
      port = hostPortPart.substring(colonHostIndex + 1);
    } else {
      host = hostPortPart;
      port = type === 'postgres' ? '5432' : type === 'mysql' ? '3306' : type === 'mongodb' ? '27017' : type === 'redis' ? '6379' : undefined;
    }

    if (pathAndQuery) {
      database = pathAndQuery.split('?')[0];
    }

    const hostLabel = host ? (host.length > 25 ? host.slice(0, 22) + '...' : host) : 'Database';
    const typeLabel = type ? type.toUpperCase() : 'Cloud DB';

    return {
      type,
      host,
      port,
      database: database || (type === 'mongodb' ? 'test' : 'neondb'),
      username,
      password,
      displayName: `${typeLabel} (${hostLabel})`
    };
  } catch {
    return { type };
  }
}

export const NewConnectionModal: React.FC<NewConnectionModalProps> = ({
  isOpen,
  onClose,
  onConnect
}) => {
  const [connectionMode, setConnectionMode] = useState<ConnectionMode>('uri');
  const [name, setName] = useState('Production PostgreSQL (Neon)');
  const [type, setType] = useState<DbType>('postgres');
  const [connectionString, setConnectionString] = useState('');
  const [host, setHost] = useState('ep-cool-dawn-123456.us-east-2.aws.neon.tech');
  const [port, setPort] = useState('5432');
  const [database, setDatabase] = useState('neondb');
  const [username, setUsername] = useState('alex');
  const [password, setPassword] = useState('');
  const [ssl, setSsl] = useState(true);
  const [isTesting, setIsTesting] = useState(false);
  const [testResult, setTestResult] = useState<{ success: boolean; message: string } | null>(null);

  if (!isOpen) return null;

  const handleUriChange = (val: string) => {
    setConnectionString(val);
    setTestResult(null);

    const parsed = parseDatabaseUri(val);
    if (parsed.type) setType(parsed.type);
    if (parsed.host) setHost(parsed.host);
    if (parsed.port) setPort(parsed.port);
    if (parsed.database) setDatabase(parsed.database);
    if (parsed.username) setUsername(parsed.username);
    if (parsed.password) setPassword(parsed.password);
    if (parsed.displayName && (!name || name.startsWith('New ') || name.includes('Database'))) {
      setName(parsed.displayName);
    }
    // Auto-enable SSL for cloud connection strings
    if (val.includes('sslmode') || val.includes('neon.tech') || val.includes('supabase') || val.includes('tidbcloud') || val.includes('upstash') || val.includes('aiven')) {
      setSsl(true);
    }
  };

  const handleTypeChange = (newType: DbType) => {
    setType(newType);
    if (newType === 'postgres') {
      setName('PostgreSQL Database');
      setPort('5432');
      setUsername('postgres');
      setDatabase('postgres');
      setSsl(true);
    } else if (newType === 'mysql') {
      setName('MySQL Database');
      setPort('3306');
      setUsername('root');
      setDatabase('app_db');
    } else if (newType === 'mongodb') {
      setName('MongoDB Cluster (Atlas)');
      setPort('27017');
      setUsername('');
      setDatabase('test');
    } else if (newType === 'sqlite') {
      setName('Local SQLite DB');
      setDatabase('app_local.sqlite');
      setSsl(false);
    } else if (newType === 'redis') {
      setName('Redis Cache');
      setPort('6379');
      setDatabase('0');
    }
    setTestResult(null);
  };

  const handleTestConnection = async () => {
    setIsTesting(true);
    setTestResult(null);

    const config = {
      type,
      host,
      port,
      database: database.trim() || (type === 'mongodb' ? 'test' : 'postgres'),
      username,
      password,
      ssl,
      connectionString: connectionString.trim() || undefined
    };

    const res = await DatabaseService.testConnection(config);
    setIsTesting(false);
    setTestResult(res);
  };

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!name.trim()) return;

    const dbConfig: DatabaseConnection = {
      id: `db-${Date.now()}`,
      name: name.trim(),
      type,
      database: database.trim() || (type === 'mongodb' ? 'test' : 'main'),
      connectionString: connectionString.trim() || undefined,
      isConnected: true,
      tables: [
        {
          name: type === 'mongodb' ? 'documents' : 'users',
          rowCount: 0,
          columns: [
            { name: '_id', type: 'OBJECTID', isPrimaryKey: true, isNullable: false },
            { name: 'createdAt', type: 'DATE', isPrimaryKey: false, isNullable: true }
          ]
        }
      ]
    };

    // Attach credentials for native drivers
    (dbConfig as any).host = host;
    (dbConfig as any).port = port;
    (dbConfig as any).username = username;
    (dbConfig as any).password = password;
    (dbConfig as any).ssl = ssl;

    // Fetch real schema from server if available
    const schemaRes = await DatabaseService.fetchSchema(dbConfig);
    if (schemaRes.success && schemaRes.tables.length > 0) {
      dbConfig.tables = schemaRes.tables;
    }

    onConnect(dbConfig);
    onClose();
  };

  return (
    <div className="modal-overlay" onClick={onClose}>
      <div className="modal-content" style={{ maxWidth: '620px' }} onClick={(e) => e.stopPropagation()}>
        {/* Modal Header */}
        <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '14px' }}>
          <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
            <Database size={20} color="#10b981" />
            <h2 style={{ fontSize: '18px', fontWeight: 700, color: '#fff' }}>
              Add Database Connection
            </h2>
          </div>

          <button onClick={onClose} className="sidebar-action-btn" style={{ padding: '4px' }}>
            <X size={16} />
          </button>
        </div>

        {/* Instant Demo Sandbox Banner */}
        <div style={{
          display: 'flex',
          justifyContent: 'space-between',
          alignItems: 'center',
          background: 'linear-gradient(135deg, rgba(16, 185, 129, 0.12), rgba(59, 130, 246, 0.08))',
          border: '1px solid rgba(16, 185, 129, 0.3)',
          borderRadius: 'var(--radius-md)',
          padding: '10px 14px',
          marginBottom: '16px'
        }}>
          <div>
            <div style={{ fontSize: '12px', fontWeight: 600, color: '#10b981', display: 'flex', alignItems: 'center', gap: '6px' }}>
              <Sparkles size={13} />
              <span>No Cloud Database? Use Free Instant Playground</span>
            </div>
            <div style={{ fontSize: '11px', color: 'var(--text-dim)', marginTop: '2px' }}>
              Instantly loads a full demo database with users, workspaces & accounts tables.
            </div>
          </div>

          <button
            type="button"
            onClick={() => {
              const demoDb: DatabaseConnection = {
                id: `db-demo-${Date.now()}`,
                name: 'Sample SaaS Production DB',
                type: 'postgres',
                database: 'saas_production_db',
                isConnected: true,
                tables: [
                  {
                    name: 'users',
                    rowCount: 1420,
                    columns: [
                      { name: 'id', type: 'VARCHAR(36)', isPrimaryKey: true, isNullable: false },
                      { name: 'email', type: 'VARCHAR(255)', isPrimaryKey: false, isNullable: false },
                      { name: 'name', type: 'VARCHAR(100)', isPrimaryKey: false, isNullable: false },
                      { name: 'role', type: 'VARCHAR(50)', isPrimaryKey: false, isNullable: false },
                      { name: 'status', type: 'VARCHAR(20)', isPrimaryKey: false, isNullable: false },
                      { name: 'created_at', type: 'TIMESTAMP', isPrimaryKey: false, isNullable: false }
                    ]
                  },
                  {
                    name: 'workspaces',
                    rowCount: 480,
                    columns: [
                      { name: 'id', type: 'VARCHAR(36)', isPrimaryKey: true, isNullable: false },
                      { name: 'name', type: 'VARCHAR(100)', isPrimaryKey: false, isNullable: false },
                      { name: 'plan_tier', type: 'VARCHAR(50)', isPrimaryKey: false, isNullable: false },
                      { name: 'storage_mb', type: 'INTEGER', isPrimaryKey: false, isNullable: false },
                      { name: 'created_at', type: 'DATE', isPrimaryKey: false, isNullable: false }
                    ]
                  },
                  {
                    name: 'accounts',
                    rowCount: 890,
                    columns: [
                      { name: 'id', type: 'VARCHAR(36)', isPrimaryKey: true, isNullable: false },
                      { name: 'user_id', type: 'VARCHAR(36)', isPrimaryKey: false, isNullable: false },
                      { name: 'balance_cents', type: 'BIGINT', isPrimaryKey: false, isNullable: false },
                      { name: 'currency', type: 'VARCHAR(3)', isPrimaryKey: false, isNullable: false },
                      { name: 'status', type: 'VARCHAR(20)', isPrimaryKey: false, isNullable: false }
                    ]
                  }
                ]
              };
              onConnect(demoDb);
              onClose();
            }}
            className="btn-secondary"
            style={{ borderColor: 'rgba(16, 185, 129, 0.4)', color: '#6ee7b7', padding: '6px 12px', fontSize: '11px', fontWeight: 600, whiteSpace: 'nowrap' }}
          >
            ⚡ Load Playground
          </button>
        </div>

        {/* Mode Selector Tabs: Connection URI vs Individual Parameters */}
        <div style={{ display: 'flex', gap: '8px', marginBottom: '16px', background: 'var(--bg-input)', padding: '3px', borderRadius: 'var(--radius-md)' }}>
          <button
            type="button"
            onClick={() => setConnectionMode('uri')}
            className={`subtab-btn ${connectionMode === 'uri' ? 'active' : ''}`}
            style={{ flex: 1, display: 'flex', alignItems: 'center', justifyContent: 'center', gap: '6px', padding: '6px', fontSize: '12px', fontWeight: 600 }}
          >
            <Link2 size={13} color="#10b981" />
            <span>Connection URI / String (Neon, Atlas, TiDB, etc.)</span>
          </button>
          <button
            type="button"
            onClick={() => setConnectionMode('params')}
            className={`subtab-btn ${connectionMode === 'params' ? 'active' : ''}`}
            style={{ flex: 1, display: 'flex', alignItems: 'center', justifyContent: 'center', gap: '6px', padding: '6px', fontSize: '12px', fontWeight: 600 }}
          >
            <Sliders size={13} color="#60a5fa" />
            <span>Individual Parameters (Host, Port, User)</span>
          </button>
        </div>

        {/* Database Driver Selector */}
        <div style={{ display: 'flex', gap: '6px', marginBottom: '16px' }}>
          {(['postgres', 'mysql', 'mongodb', 'sqlite', 'redis'] as DbType[]).map((t) => (
            <button
              key={t}
              type="button"
              onClick={() => handleTypeChange(t)}
              className={`subtab-btn ${type === t ? 'active' : ''}`}
              style={{ flex: 1, padding: '7px 2px', textTransform: 'uppercase', fontWeight: 700, fontSize: '10px' }}
            >
              {t}
            </button>
          ))}
        </div>

        <form onSubmit={handleSubmit}>
          <div style={{ display: 'flex', flexDirection: 'column', gap: '12px' }}>
            {/* Display Name */}
            <div>
              <label style={{ display: 'block', fontSize: '11px', color: 'var(--text-dim)', marginBottom: '4px' }}>
                Connection Display Name
              </label>
              <input
                type="text"
                value={name}
                onChange={(e) => setName(e.target.value)}
                placeholder="e.g. Production Neon Postgres, Atlas MongoDB Cluster"
                className="url-input"
                style={{
                  width: '100%',
                  background: 'var(--bg-input)',
                  border: '1px solid var(--border-subtle)',
                  borderRadius: 'var(--radius-md)',
                  padding: '8px 12px',
                  color: '#fff'
                }}
              />
            </div>

            {/* Connection URI Mode */}
            {connectionMode === 'uri' && (
              <div>
                <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '4px' }}>
                  <label style={{ fontSize: '11px', color: 'var(--text-dim)' }}>
                    Database Connection URI / URL
                  </label>
                  <span style={{ fontSize: '10px', color: '#10b981', display: 'flex', alignItems: 'center', gap: '4px' }}>
                    <Sparkles size={11} /> Auto-detects Host, Port, User & DB
                  </span>
                </div>
                <textarea
                  value={connectionString}
                  onChange={(e) => handleUriChange(e.target.value)}
                  placeholder={
                    type === 'postgres'
                      ? 'postgresql://user:password@ep-cool-dawn.us-east-2.aws.neon.tech/neondb?sslmode=require'
                      : type === 'mongodb'
                      ? 'mongodb+srv://devuser:password123@cluster0.abcde.mongodb.net/production?retryWrites=true&w=majority'
                      : type === 'mysql'
                      ? 'mysql://root:password@gateway01.us-east-1.tidbcloud.com:4000/app_db'
                      : type === 'redis'
                      ? 'rediss://default:token@us1-cool-panda.upstash.io:6379'
                      : 'sqlite:///path/to/database.db'
                  }
                  rows={3}
                  className="code-textarea"
                  style={{
                    width: '100%',
                    background: 'var(--bg-input)',
                    border: '1px solid var(--border-subtle)',
                    borderRadius: 'var(--radius-md)',
                    padding: '8px 12px',
                    color: '#fff',
                    fontFamily: 'var(--font-mono)',
                    fontSize: '11px'
                  }}
                />

                {/* Parsed summary chip */}
                {host && (
                  <div style={{ display: 'flex', flexWrap: 'wrap', gap: '6px', marginTop: '6px', fontSize: '10px', color: 'var(--text-dim)' }}>
                    <span className="method-pill method-GET" style={{ fontSize: '9px', textTransform: 'uppercase' }}>{type}</span>
                    <span>Host: <strong style={{ color: '#fff' }}>{host}</strong></span>
                    {port && <span>Port: <strong style={{ color: '#fff' }}>{port}</strong></span>}
                    {database && <span>DB: <strong style={{ color: '#38bdf8' }}>{database}</strong></span>}
                    {username && <span>User: <strong style={{ color: '#a855f7' }}>{username}</strong></span>}
                  </div>
                )}
              </div>
            )}

            {/* Individual Parameters Mode */}
            {connectionMode === 'params' && (
              <>
                {type !== 'sqlite' && (
                  <div style={{ display: 'grid', gridTemplateColumns: '2fr 1fr', gap: '10px' }}>
                    <div>
                      <label style={{ display: 'block', fontSize: '11px', color: 'var(--text-dim)', marginBottom: '4px' }}>
                        Host / Server Address
                      </label>
                      <input
                        type="text"
                        value={host}
                        onChange={(e) => setHost(e.target.value)}
                        placeholder="localhost or ep-xyz.neon.tech"
                        className="url-input"
                        style={{
                          width: '100%',
                          background: 'var(--bg-input)',
                          border: '1px solid var(--border-subtle)',
                          borderRadius: 'var(--radius-md)',
                          padding: '8px 12px',
                          color: '#fff',
                          fontFamily: 'var(--font-mono)'
                        }}
                      />
                    </div>

                    <div>
                      <label style={{ display: 'block', fontSize: '11px', color: 'var(--text-dim)', marginBottom: '4px' }}>
                        Port
                      </label>
                      <input
                        type="text"
                        value={port}
                        onChange={(e) => setPort(e.target.value)}
                        className="url-input"
                        style={{
                          width: '100%',
                          background: 'var(--bg-input)',
                          border: '1px solid var(--border-subtle)',
                          borderRadius: 'var(--radius-md)',
                          padding: '8px 12px',
                          color: '#fff',
                          fontFamily: 'var(--font-mono)'
                        }}
                      />
                    </div>
                  </div>
                )}

                <div style={{ display: 'grid', gridTemplateColumns: type === 'sqlite' ? '1fr' : '1fr 1fr', gap: '10px' }}>
                  <div>
                    <label style={{ display: 'block', fontSize: '11px', color: 'var(--text-dim)', marginBottom: '4px' }}>
                      {type === 'sqlite' ? 'SQLite Database File Path' : 'Database Name'}
                    </label>
                    <input
                      type="text"
                      value={database}
                      onChange={(e) => setDatabase(e.target.value)}
                      placeholder={type === 'sqlite' ? 'local_app.sqlite' : 'production_db'}
                      className="url-input"
                      style={{
                        width: '100%',
                        background: 'var(--bg-input)',
                        border: '1px solid var(--border-subtle)',
                        borderRadius: 'var(--radius-md)',
                        padding: '8px 12px',
                        color: '#fff',
                        fontFamily: 'var(--font-mono)'
                      }}
                    />
                  </div>

                  {type !== 'sqlite' && (
                    <div>
                      <label style={{ display: 'block', fontSize: '11px', color: 'var(--text-dim)', marginBottom: '4px' }}>
                        Username
                      </label>
                      <input
                        type="text"
                        value={username}
                        onChange={(e) => setUsername(e.target.value)}
                        className="url-input"
                        style={{
                          width: '100%',
                          background: 'var(--bg-input)',
                          border: '1px solid var(--border-subtle)',
                          borderRadius: 'var(--radius-md)',
                          padding: '8px 12px',
                          color: '#fff',
                          fontFamily: 'var(--font-mono)'
                        }}
                      />
                    </div>
                  )}
                </div>

                {type !== 'sqlite' && (
                  <div>
                    <label style={{ display: 'block', fontSize: '11px', color: 'var(--text-dim)', marginBottom: '4px' }}>
                      Password (Stored in OS Secure Keyring)
                    </label>
                    <input
                      type="password"
                      value={password}
                      onChange={(e) => setPassword(e.target.value)}
                      placeholder="••••••••••••"
                      className="url-input"
                      style={{
                        width: '100%',
                        background: 'var(--bg-input)',
                        border: '1px solid var(--border-subtle)',
                        borderRadius: 'var(--radius-md)',
                        padding: '8px 12px',
                        color: '#fff'
                      }}
                    />
                  </div>
                )}
              </>
            )}

            {/* SSL / TLS Toggle for Cloud Databases */}
            {type !== 'sqlite' && (
              <div style={{ display: 'flex', alignItems: 'center', gap: '8px', padding: '4px 0' }}>
                <input
                  type="checkbox"
                  id="db-ssl-toggle"
                  checked={ssl}
                  onChange={(e) => setSsl(e.target.checked)}
                  style={{ width: '14px', height: '14px', accentColor: '#10b981', cursor: 'pointer' }}
                />
                <label htmlFor="db-ssl-toggle" style={{ fontSize: '11px', color: '#fff', cursor: 'pointer', display: 'flex', alignItems: 'center', gap: '4px' }}>
                  <Shield size={12} color="#10b981" />
                  <span>Enable SSL / TLS Encryption (Required for Neon, Supabase, Cloud DBs)</span>
                </label>
              </div>
            )}

            {/* Test Connection Diagnostic Result */}
            {testResult && (
              <div style={{
                padding: '10px 14px',
                borderRadius: 'var(--radius-sm)',
                background: testResult.success ? 'rgba(16, 185, 129, 0.1)' : 'rgba(239, 68, 68, 0.1)',
                border: `1px solid ${testResult.success ? 'rgba(16, 185, 129, 0.3)' : 'rgba(239, 68, 68, 0.3)'}`,
                fontSize: '12px',
                color: testResult.success ? '#10b981' : '#ef4444'
              }}>
                {testResult.message}
              </div>
            )}
          </div>

          {/* Action Buttons */}
          <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginTop: '20px' }}>
            <button
              type="button"
              onClick={handleTestConnection}
              disabled={isTesting}
              className="btn-secondary"
            >
              <Zap size={13} color="#f59e0b" />
              <span>{isTesting ? 'Pinging Database...' : 'Test Connection'}</span>
            </button>

            <div style={{ display: 'flex', gap: '8px' }}>
              <button type="button" onClick={onClose} className="btn-secondary">
                Cancel
              </button>
              <button type="submit" className="btn-send" style={{ padding: '8px 18px' }}>
                <Check size={14} />
                <span>Save & Connect</span>
              </button>
            </div>
          </div>
        </form>
      </div>
    </div>
  );
};
