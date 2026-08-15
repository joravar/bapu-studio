import React, { useState } from 'react';
import { X, Database, Check, Zap, Server, Shield, HardDrive } from 'lucide-react';
import { DatabaseConnection } from '../../types';

interface NewConnectionModalProps {
  isOpen: boolean;
  onClose: () => void;
  onConnect: (connection: DatabaseConnection) => void;
}

type DbType = 'postgres' | 'mysql' | 'mongodb' | 'sqlite' | 'redis';

import { DatabaseService } from '../../services/databaseService';

export const NewConnectionModal: React.FC<NewConnectionModalProps> = ({
  isOpen,
  onClose,
  onConnect
}) => {
  const [name, setName] = useState('New PostgreSQL Connection');
  const [type, setType] = useState<DbType>('postgres');
  const [host, setHost] = useState('localhost');
  const [port, setPort] = useState('5432');
  const [database, setDatabase] = useState('my_app_db');
  const [username, setUsername] = useState('postgres');
  const [password, setPassword] = useState('');
  const [connectionString, setConnectionString] = useState('');
  const [isTesting, setIsTesting] = useState(false);
  const [testResult, setTestResult] = useState<{ success: boolean; message: string } | null>(null);

  if (!isOpen) return null;

  const handleTypeChange = (newType: DbType) => {
    setType(newType);
    if (newType === 'postgres') {
      setName('PostgreSQL Database');
      setPort('5432');
      setUsername('postgres');
    } else if (newType === 'mysql') {
      setName('MySQL Database');
      setPort('3306');
      setUsername('root');
    } else if (newType === 'mongodb') {
      setName('MongoDB Cluster / Local');
      setPort('27017');
      setUsername('');
      setDatabase('app_db');
    } else if (newType === 'sqlite') {
      setName('Local SQLite DB');
      setDatabase('app_local.sqlite');
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

    // Attach credentials
    (dbConfig as any).host = host;
    (dbConfig as any).port = port;
    (dbConfig as any).username = username;
    (dbConfig as any).password = password;

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
      <div className="modal-content" style={{ maxWidth: '580px' }} onClick={(e) => e.stopPropagation()}>
        {/* Header */}
        <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '16px' }}>
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

        {/* Database Driver Selector */}
        <div style={{ display: 'flex', gap: '6px', marginBottom: '16px' }}>
          {(['postgres', 'mysql', 'mongodb', 'sqlite', 'redis'] as DbType[]).map((t) => (
            <button
              key={t}
              type="button"
              onClick={() => handleTypeChange(t)}
              className={`subtab-btn ${type === t ? 'active' : ''}`}
              style={{ flex: 1, padding: '8px 2px', textTransform: 'uppercase', fontWeight: 700, fontSize: '10px' }}
            >
              {t}
            </button>
          ))}
        </div>

        <form onSubmit={handleSubmit}>
          <div style={{ display: 'flex', flexDirection: 'column', gap: '12px' }}>
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

            {type === 'mongodb' && (
              <div>
                <label style={{ display: 'block', fontSize: '11px', color: 'var(--text-dim)', marginBottom: '4px' }}>
                  MongoDB Connection URI (Optional — e.g. Atlas <code>mongodb+srv://...</code>)
                </label>
                <input
                  type="text"
                  value={connectionString}
                  onChange={(e) => setConnectionString(e.target.value)}
                  placeholder="mongodb+srv://user:pass@cluster.mongodb.net/dbname"
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

            {type !== 'sqlite' && !connectionString && (
              <div style={{ display: 'grid', gridTemplateColumns: '2fr 1fr', gap: '10px' }}>
                <div>
                  <label style={{ display: 'block', fontSize: '11px', color: 'var(--text-dim)', marginBottom: '4px' }}>
                    Host / IP
                  </label>
                  <input
                    type="text"
                    value={host}
                    onChange={(e) => setHost(e.target.value)}
                    placeholder="localhost or db.example.com"
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
                  {type === 'sqlite' ? 'SQLite Database File Name' : 'Database Name'}
                </label>
                <input
                  type="text"
                  value={database}
                  onChange={(e) => setDatabase(e.target.value)}
                  placeholder="database_name"
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

          <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginTop: '20px' }}>
            <button
              type="button"
              onClick={handleTestConnection}
              disabled={isTesting}
              className="btn-secondary"
            >
              <Zap size={13} color="#f59e0b" />
              <span>{isTesting ? 'Pinging...' : 'Test Connection'}</span>
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
