import React, { useState } from 'react';
import { 
  Play, 
  Table, 
  Key, 
  Download, 
  Sparkles, 
  Clock, 
  Check, 
  Layers,
  Database,
  Search
} from 'lucide-react';
import { DatabaseConnection, TableSchema } from '../../types';
import { SqliteDropZone } from './SqliteDropZone';
import { AiCopilotModal } from '../AiCopilot/AiCopilotModal';
import { DatabaseService } from '../../services/databaseService';

interface DatabaseStudioProps {
  activeDb: DatabaseConnection;
  onRecordHistory: (title: string, subtitle: string, status?: number) => void;
  onDatabaseLoaded?: (db: DatabaseConnection) => void;
}

interface QueryResult {
  columns: string[];
  rows: any[];
  rowCount: number;
  executionTimeMs: number;
  error?: string;
}

export const DatabaseStudio: React.FC<DatabaseStudioProps> = ({
  activeDb,
  onRecordHistory,
  onDatabaseLoaded
}) => {
  const [selectedTable, setSelectedTable] = useState<TableSchema | null>(
    activeDb.tables[0] || null
  );
  const [sqlQuery, setSqlQuery] = useState<string>(
    'SELECT * FROM users ORDER BY created_at DESC LIMIT 15;'
  );
  const [isExecuting, setIsExecuting] = useState(false);
  const [searchFilter, setSearchFilter] = useState('');
  const [isAiModalOpen, setIsAiModalOpen] = useState(false);

  // Sample dynamic query results
  const [queryResult, setQueryResult] = useState<QueryResult>({
    columns: ['id', 'email', 'organization_id', 'role', 'created_at'],
    rows: [
      { id: 'u_101', email: 'sarah.connor@acme.dev', organization_id: 'org_881', role: 'admin', created_at: '2026-02-10 14:22:01' },
      { id: 'u_102', email: 'marcus.vance@stripe-integrations.io', organization_id: 'org_882', role: 'developer', created_at: '2026-02-11 09:15:30' },
      { id: 'u_103', email: 'elena.rostova@cloudscale.net', organization_id: 'org_881', role: 'owner', created_at: '2026-02-12 18:40:12' },
      { id: 'u_104', email: 'david.kim@fintech-ai.com', organization_id: 'org_883', role: 'developer', created_at: '2026-02-13 11:05:44' },
      { id: 'u_105', email: 'priya.sharma@hyperloop.io', organization_id: 'org_884', role: 'billing', created_at: '2026-02-14 16:30:19' }
    ],
    rowCount: 5,
    executionTimeMs: 14
  });

  const handleExecuteSql = async () => {
    setIsExecuting(true);
    const result = await DatabaseService.executeQuery(activeDb, sqlQuery);
    setIsExecuting(false);

    if (result.success) {
      setQueryResult({
        columns: result.columns,
        rows: result.rows,
        rowCount: result.rowCount,
        executionTimeMs: result.executionTimeMs
      });
      onRecordHistory(`SQL: ${sqlQuery.substring(0, 30)}...`, `${result.rowCount} rows • ${result.executionTimeMs}ms`);
    } else {
      setQueryResult(prev => ({
        ...prev,
        error: result.message,
        executionTimeMs: result.executionTimeMs
      }));
      onRecordHistory(`SQL Error: ${sqlQuery.substring(0, 25)}`, result.message || 'Execution error');
    }
  };

  const handleExportCsv = () => {
    const header = queryResult.columns.join(',');
    const rows = queryResult.rows.map(r => queryResult.columns.map(col => JSON.stringify(r[col] || '')).join(','));
    const csvContent = 'data:text/csv;charset=utf-8,' + [header, ...rows].join('\n');
    const encodedUri = encodeURI(csvContent);
    const link = document.createElement('a');
    link.setAttribute('href', encodedUri);
    link.setAttribute('download', `${activeDb.database}_query_export.csv`);
    document.body.appendChild(link);
    link.click();
    document.body.removeChild(link);
  };

  const filteredRows = queryResult.rows.filter(row => {
    if (!searchFilter) return true;
    return Object.values(row).some(val => 
      String(val).toLowerCase().includes(searchFilter.toLowerCase())
    );
  });

  return (
    <div className="db-layout">
      {/* Left Database Schema Browser */}
      <aside className="db-schema-sidebar">
        <div style={{ display: 'flex', alignItems: 'center', gap: '6px', marginBottom: '12px' }}>
          <Database size={15} color="#10b981" />
          <span style={{ fontWeight: 700, fontSize: '12px' }}>{activeDb.database}</span>
        </div>

        {/* Local SQLite Drag and Drop Zone */}
        {onDatabaseLoaded && (
          <SqliteDropZone onDatabaseLoaded={onDatabaseLoaded} />
        )}

        <div className="sidebar-section-header">Tables ({activeDb.tables.length})</div>
        {activeDb.tables.map(table => (
          <div key={table.name} style={{ marginBottom: '8px' }}>
            <div
              onClick={() => {
                setSelectedTable(table);
                setSqlQuery(`SELECT * FROM ${table.name} LIMIT 25;`);
              }}
              style={{
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'space-between',
                padding: '5px 8px',
                borderRadius: 'var(--radius-sm)',
                cursor: 'pointer',
                background: selectedTable?.name === table.name ? 'var(--bg-card-hover)' : 'transparent',
                color: selectedTable?.name === table.name ? '#fff' : 'var(--text-muted)'
              }}
            >
              <div style={{ display: 'flex', alignItems: 'center', gap: '6px' }}>
                <Table size={13} color="#60a5fa" />
                <span style={{ fontSize: '12px', fontWeight: 500 }}>{table.name}</span>
              </div>
              <span style={{ fontSize: '10px', color: 'var(--text-dim)', fontFamily: 'var(--font-mono)' }}>
                {table.rowCount.toLocaleString()}
              </span>
            </div>

            {selectedTable?.name === table.name && (
              <div style={{ paddingLeft: '18px', marginTop: '4px' }}>
                {table.columns.map(col => (
                  <div key={col.name} style={{ display: 'flex', alignItems: 'center', gap: '6px', fontSize: '11px', color: 'var(--text-dim)', padding: '2px 0' }}>
                    {col.isPrimaryKey ? <Key size={10} color="#f59e0b" /> : <div style={{ width: '10px' }} />}
                    <span style={{ color: col.isPrimaryKey ? 'var(--text-main)' : 'var(--text-muted)', fontFamily: 'var(--font-mono)' }}>{col.name}</span>
                    <span style={{ fontSize: '9px', color: 'var(--text-dim)' }}>{col.type}</span>
                  </div>
                ))}
              </div>
            )}
          </div>
        ))}
      </aside>

      {/* Main SQL / Mongo Console & Results Pane */}
      <div className="db-main-pane">
        {/* Top Query Editor Box */}
        <div className="sql-editor-box">
          <textarea
            value={sqlQuery}
            onChange={(e) => setSqlQuery(e.target.value)}
            className="code-textarea"
            spellCheck={false}
            placeholder={activeDb.type === 'mongodb' 
              ? 'Write MongoDB JSON query or MQL (e.g. { "status": "active" } or users.find({}))'
              : 'Write SQL query here... (e.g. SELECT * FROM users;)'}
          />

          <div className="sql-actions-bar">
            <div style={{ display: 'flex', gap: '6px' }}>
              {activeDb.type === 'mongodb' ? (
                <>
                  <button 
                    onClick={() => setSqlQuery('{}')}
                    className="btn-secondary"
                    style={{ fontSize: '11px', padding: '3px 8px' }}
                  >
                    Find All ({'{}'})
                  </button>
                  <button 
                    onClick={() => setSqlQuery('users.find({ "status": "active" })')}
                    className="btn-secondary"
                    style={{ fontSize: '11px', padding: '3px 8px' }}
                  >
                    Find Active Users
                  </button>
                </>
              ) : (
                <>
                  <button 
                    onClick={() => setSqlQuery('SELECT * FROM users ORDER BY created_at DESC LIMIT 20;')}
                    className="btn-secondary"
                    style={{ fontSize: '11px', padding: '3px 8px' }}
                  >
                    SELECT * Users
                  </button>
                  <button 
                    onClick={() => setSqlQuery('SELECT COUNT(*), role FROM users GROUP BY role;')}
                    className="btn-secondary"
                    style={{ fontSize: '11px', padding: '3px 8px' }}
                  >
                    Count by Role
                  </button>
                </>
              )}
            </div>

            <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
              <button
                onClick={() => setIsAiModalOpen(true)}
                className="btn-secondary"
                style={{ borderColor: 'rgba(16, 185, 129, 0.3)', color: '#6ee7b7' }}
                title="Generate SQL from Natural Language"
              >
                <Sparkles size={13} color="#10b981" />
                <span>AI SQL Copilot</span>
              </button>

              <button 
                onClick={handleExecuteSql}
                disabled={isExecuting}
                className="btn-send"
                style={{ padding: '6px 14px', fontSize: '12px' }}
              >
                <Play size={13} />
                <span>{isExecuting ? 'Running...' : 'Run Query (F5)'}</span>
              </button>
            </div>
          </div>
        </div>

        {/* Results Toolbar */}
        <div className="panel-tab-header">
          <div style={{ display: 'flex', alignItems: 'center', gap: '12px' }}>
            <span style={{ fontSize: '12px', fontWeight: 600, color: 'var(--text-main)' }}>
              Query Results
            </span>
            <span style={{ fontSize: '11px', color: 'var(--text-muted)', fontFamily: 'var(--font-mono)' }}>
              {queryResult.rowCount} rows in {queryResult.executionTimeMs}ms
            </span>
          </div>

          <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
            <div style={{ display: 'flex', alignItems: 'center', gap: '6px', background: 'var(--bg-input)', padding: '2px 8px', borderRadius: 'var(--radius-sm)', border: '1px solid var(--border-subtle)' }}>
              <Search size={11} color="var(--text-dim)" />
              <input
                type="text"
                value={searchFilter}
                onChange={(e) => setSearchFilter(e.target.value)}
                placeholder="Filter table rows..."
                style={{ background: 'transparent', border: 'none', color: '#fff', fontSize: '11px', outline: 'none', width: '120px' }}
              />
            </div>

            <button onClick={handleExportCsv} className="btn-secondary" style={{ fontSize: '11px', padding: '3px 8px' }}>
              <Download size={12} />
              <span>Export CSV</span>
            </button>
          </div>
        </div>

        {/* Error Diagnostic Alert */}
        {queryResult.error && (
          <div style={{
            margin: '8px 12px',
            padding: '10px 14px',
            background: 'rgba(239, 68, 68, 0.1)',
            border: '1px solid rgba(239, 68, 68, 0.3)',
            borderRadius: 'var(--radius-sm)',
            fontSize: '12px',
            color: '#f87171',
            fontFamily: 'var(--font-mono)'
          }}>
            ⚠️ {queryResult.error}
          </div>
        )}

        {/* Data Results Grid */}
        <div className="data-grid-container">
          <table className="data-grid-table">
            <thead>
              <tr>
                {queryResult.columns.map(col => (
                  <th key={col}>{col}</th>
                ))}
              </tr>
            </thead>
            <tbody>
              {filteredRows.map((row, rIdx) => (
                <tr key={rIdx}>
                  {queryResult.columns.map(col => (
                    <td key={col}>
                      {String(row[col] ?? 'NULL')}
                    </td>
                  ))}
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </div>

      {/* AI Copilot Modal for SQL */}
      <AiCopilotModal
        isOpen={isAiModalOpen}
        onClose={() => setIsAiModalOpen(false)}
        onApplySql={(generatedSql) => {
          setSqlQuery(generatedSql);
        }}
      />
    </div>
  );
};
