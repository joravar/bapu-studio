import React, { useState, useEffect } from 'react';
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
  Search,
  Settings
} from 'lucide-react';
import { DatabaseConnection, TableSchema } from '../../types';
import { SqliteDropZone } from './SqliteDropZone';
import { AiCopilotModal } from '../AiCopilot/AiCopilotModal';
import { NewConnectionModal } from './NewConnectionModal';
import { DatabaseService } from '../../services/databaseService';
import { SAMPLE_PLAYGROUND_DB } from '../../data/mockData';

interface DatabaseStudioProps {
  activeDb: DatabaseConnection;
  onRecordHistory: (title: string, subtitle: string, status?: number) => void;
  onDatabaseLoaded?: (db: DatabaseConnection) => void;
  onRenameDatabase?: (dbId: string, newName: string) => void;
  onUpdateDatabase?: (db: DatabaseConnection) => void;
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
  onDatabaseLoaded,
  onRenameDatabase,
  onUpdateDatabase
}) => {
  const [selectedTable, setSelectedTable] = useState<TableSchema | null>(
    activeDb.tables[0] || null
  );
  const [sqlQuery, setSqlQuery] = useState<string>('');
  const [isExecuting, setIsExecuting] = useState(false);
  const [searchFilter, setSearchFilter] = useState('');
  const [isAiModalOpen, setIsAiModalOpen] = useState(false);
  const [isEditModalOpen, setIsEditModalOpen] = useState(false);

  // Resizable schema sidebar state
  const [dbSidebarWidth, setDbSidebarWidth] = useState<number>(() => {
    try {
      const saved = localStorage.getItem('bapu_db_sidebar_width');
      if (saved) return Math.max(160, Math.min(480, Number(saved)));
    } catch {}
    return 220;
  });
  const [isResizingDb, setIsResizingDb] = useState(false);

  useEffect(() => {
    if (!isResizingDb) return;

    const handleMouseMove = (e: MouseEvent) => {
      // Calculate based on clientX offset from window left
      const newWidth = Math.max(160, Math.min(480, e.clientX - 260));
      setDbSidebarWidth(newWidth);
      try {
        localStorage.setItem('bapu_db_sidebar_width', String(newWidth));
      } catch {}
    };

    const handleMouseUp = () => {
      setIsResizingDb(false);
    };

    document.addEventListener('mousemove', handleMouseMove);
    document.addEventListener('mouseup', handleMouseUp);
    return () => {
      document.removeEventListener('mousemove', handleMouseMove);
      document.removeEventListener('mouseup', handleMouseUp);
    };
  }, [isResizingDb]);

  // Query results state
  const [queryResult, setQueryResult] = useState<QueryResult>({
    columns: [],
    rows: [],
    rowCount: 0,
    executionTimeMs: 0
  });

  // Auto-sync when active database changes
  useEffect(() => {
    const firstTable = activeDb?.tables?.[0];
    if (firstTable && activeDb.id !== 'db-empty') {
      setSelectedTable(firstTable);
      const query = activeDb.type === 'mongodb' 
        ? `${firstTable.name}.find({})` 
        : `SELECT * FROM ${firstTable.name} LIMIT 15;`;
      setSqlQuery(query);
      DatabaseService.executeQuery(activeDb, query).then(res => {
        if (res.success) {
          setQueryResult({
            columns: res.columns,
            rows: res.rows,
            rowCount: res.rowCount,
            executionTimeMs: res.executionTimeMs
          });
        }
      });
    } else {
      setSelectedTable(null);
      setSqlQuery('');
      setQueryResult({
        columns: [],
        rows: [],
        rowCount: 0,
        executionTimeMs: 0
      });
    }
  }, [activeDb.id]);

  const handleExecuteSql = async (overrideQuery?: string) => {
    const queryToRun = overrideQuery || sqlQuery;
    setIsExecuting(true);
    const result = await DatabaseService.executeQuery(activeDb, queryToRun);
    setIsExecuting(false);

    if (result.success) {
      setQueryResult({
        columns: result.columns,
        rows: result.rows,
        rowCount: result.rowCount,
        executionTimeMs: result.executionTimeMs
      });
      onRecordHistory(`SQL: ${queryToRun.substring(0, 30)}...`, `${result.rowCount} rows • ${result.executionTimeMs}ms`);
    } else {
      setQueryResult(prev => ({
        ...prev,
        error: result.message,
        executionTimeMs: result.executionTimeMs
      }));
      onRecordHistory(`SQL Error: ${queryToRun.substring(0, 25)}`, result.message || 'Execution error');
    }
  };

  const handleExportCsv = () => {
    if (!queryResult.rows || queryResult.rows.length === 0) return;
    const header = queryResult.columns.join(',');
    const rows = queryResult.rows.map(r => queryResult.columns.map(col => JSON.stringify(r[col] || '')).join(','));
    const csvContent = 'data:text/csv;charset=utf-8,' + [header, ...rows].join('\n');
    const encodedUri = encodeURI(csvContent);
    const link = document.createElement('a');
    link.setAttribute('href', encodedUri);
    link.setAttribute('download', `${activeDb.name.toLowerCase().replace(/\s+/g, '_')}_export.csv`);
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
    <div className="db-layout" style={{ userSelect: isResizingDb ? 'none' : 'auto' }}>
      {/* Left Database Schema Browser */}
      <aside 
        className="db-schema-sidebar"
        style={{ width: `${dbSidebarWidth}px`, flexShrink: 0, borderRight: 'none' }}
      >
        <div style={{ display: 'flex', alignItems: 'center', gap: '6px', marginBottom: '12px' }}>
          <Database size={15} color="#10b981" style={{ flexShrink: 0 }} />
          <input
            type="text"
            value={activeDb.name}
            onChange={(e) => {
              if (onRenameDatabase) onRenameDatabase(activeDb.id, e.target.value);
            }}
            placeholder="Connection Name"
            className="request-title-input"
            style={{ fontSize: '12px', fontWeight: 700, padding: '2px 6px', width: '100%', minWidth: '100px' }}
            title="Click to rename connection"
          />
          <button
            onClick={() => setIsEditModalOpen(true)}
            className="sidebar-action-btn"
            title="Edit Database Connection Parameters & Credentials"
            style={{ padding: '4px', flexShrink: 0 }}
          >
            <Settings size={13} color="var(--text-muted)" />
          </button>
        </div>

        {/* Local SQLite Drag and Drop Zone */}
        {onDatabaseLoaded && (
          <SqliteDropZone onDatabaseLoaded={onDatabaseLoaded} />
        )}

        <div className="sidebar-section-header">Tables ({activeDb.tables.length})</div>
        {activeDb.tables.length === 0 ? (
          <div style={{ fontSize: '11px', color: 'var(--text-dim)', padding: '16px 8px', lineHeight: 1.4, textAlign: 'center' }}>
            <p style={{ marginBottom: '10px' }}>
              {activeDb.id === 'db-empty' ? 'No database connection selected.' : 'No tables discovered.'}
            </p>
            {onDatabaseLoaded && (
              <button
                onClick={() => onDatabaseLoaded(SAMPLE_PLAYGROUND_DB)}
                className="btn-secondary"
                style={{ fontSize: '11px', padding: '6px 10px', color: '#10b981', borderColor: 'rgba(16, 185, 129, 0.3)', width: '100%' }}
              >
                ⚡ Load Sample Playground DB
              </button>
            )}
          </div>
        ) : (
          activeDb.tables.map(table => (
            <div key={table.name} style={{ marginBottom: '8px' }}>
              <div
                onClick={() => {
                  setSelectedTable(table);
                  const query = activeDb.type === 'mongodb' 
                    ? `${table.name}.find({})` 
                    : `SELECT * FROM ${table.name} LIMIT 25;`;
                  setSqlQuery(query);
                  handleExecuteSql(query);
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
          ))
        )}
      </aside>

      {/* Vertical Resize Handle between Schema Browser and Main Query Pane */}
      <div
        className={`pane-resizer-vertical ${isResizingDb ? 'resizing' : ''}`}
        onMouseDown={() => setIsResizingDb(true)}
        onDoubleClick={() => {
          setDbSidebarWidth(220);
          localStorage.setItem('bapu_db_sidebar_width', '220');
        }}
        title="Drag to resize schema browser • Double-click to reset (220px)"
      />

      {/* Main SQL / Mongo Console & Results Pane */}
      <div className="db-main-pane">
        {/* Top Query Editor Box */}
        <div className="sql-editor-box">
          <textarea
            value={sqlQuery}
            onChange={(e) => setSqlQuery(e.target.value)}
            onKeyDown={(e) => {
              if ((e.ctrlKey || e.metaKey) && e.key === 'Enter') {
                e.preventDefault();
                handleExecuteSql();
              } else if (e.key === 'F5') {
                e.preventDefault();
                handleExecuteSql();
              }
            }}
            className="code-textarea"
            spellCheck={false}
            placeholder={activeDb.type === 'mongodb' 
              ? 'Write MongoDB JSON query or MQL (e.g. { "status": "active" } or users.find({}))'
              : 'Write SQL query here... (e.g. SELECT * FROM table;) • Press Ctrl+Enter to run'}
          />

          <div className="sql-actions-bar" style={{ flexWrap: 'wrap', gap: '6px' }}>
            <div style={{ display: 'flex', gap: '6px', flexWrap: 'wrap', alignItems: 'center' }}>
              {selectedTable && (
                activeDb.type === 'mongodb' ? (
                  <>
                    <button 
                      onClick={() => {
                        const q = `${selectedTable.name}.find({})`;
                        setSqlQuery(q);
                        handleExecuteSql(q);
                      }}
                      className="btn-secondary"
                      style={{ fontSize: '11px', padding: '3px 8px' }}
                    >
                      Find All ({selectedTable.name})
                    </button>
                    <button 
                      onClick={() => {
                        const catCol = selectedTable.columns.find(c => /status|role|type|category|plan/i.test(c.name))?.name || 'status';
                        const q = `${selectedTable.name}.aggregate([\n  { $group: { _id: "$${catCol}", count: { $sum: 1 } } },\n  { $sort: { count: -1 } }\n])`;
                        setSqlQuery(q);
                        handleExecuteSql(q);
                      }}
                      className="btn-secondary"
                      style={{ fontSize: '11px', padding: '3px 8px' }}
                    >
                      📊 Group & Count
                    </button>
                  </>
                ) : (
                  <>
                    <button 
                      onClick={() => {
                        const dateCol = selectedTable.columns.find(c => /date|time|created|updated|at$/i.test(c.name))?.name || (selectedTable.columns.find(c => c.isPrimaryKey)?.name || 'id');
                        const q = `SELECT * FROM ${selectedTable.name} ORDER BY ${dateCol} DESC LIMIT 25;`;
                        setSqlQuery(q);
                        handleExecuteSql(q);
                      }}
                      className="btn-secondary"
                      style={{ fontSize: '11px', padding: '3px 8px' }}
                      title="Fetch recent rows ordered by date/ID"
                    >
                      ⏱️ Recent
                    </button>
                    <button 
                      onClick={() => {
                        const catCol = selectedTable.columns.find(c => /status|role|type|category|plan|state/i.test(c.name))?.name || (selectedTable.columns.length > 1 ? selectedTable.columns[1].name : 'id');
                        const q = `SELECT ${catCol}, COUNT(*) as total FROM ${selectedTable.name} GROUP BY ${catCol} ORDER BY total DESC;`;
                        setSqlQuery(q);
                        handleExecuteSql(q);
                      }}
                      className="btn-secondary"
                      style={{ fontSize: '11px', padding: '3px 8px' }}
                      title="Group and count records"
                    >
                      📊 Group & Count
                    </button>
                    <button 
                      onClick={() => {
                        const nullCol = selectedTable.columns.find(c => !c.isPrimaryKey)?.name || 'id';
                        const q = `SELECT * FROM ${selectedTable.name} WHERE ${nullCol} IS NULL;`;
                        setSqlQuery(q);
                        handleExecuteSql(q);
                      }}
                      className="btn-secondary"
                      style={{ fontSize: '11px', padding: '3px 8px' }}
                      title="Find records with NULL values"
                    >
                      🔍 Find Nulls
                    </button>
                    <button 
                      onClick={() => {
                        const q = `SELECT ${selectedTable.columns.map(c => c.name).join(', ')} FROM ${selectedTable.name} LIMIT 50;`;
                        setSqlQuery(q);
                        handleExecuteSql(q);
                      }}
                      className="btn-secondary"
                      style={{ fontSize: '11px', padding: '3px 8px' }}
                      title="Select all explicit columns"
                    >
                      📋 Column List
                    </button>
                  </>
                )
              )}
            </div>

            <div style={{ display: 'flex', alignItems: 'center', gap: '8px', marginLeft: 'auto' }}>
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
                onClick={() => handleExecuteSql()}
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
          {queryResult.columns.length === 0 ? (
            <div style={{ display: 'flex', flexDirection: 'column', alignItems: 'center', justifyContent: 'center', height: '100%', color: 'var(--text-dim)', gap: '8px', padding: '40px' }}>
              <Database size={24} style={{ opacity: 0.3 }} />
              <span style={{ fontSize: '12px' }}>
                {activeDb.id === 'db-empty'
                  ? 'No database connected. Add a connection in the sidebar or drop a SQLite file.'
                  : 'Write a query and press Run Query (F5) to view results'}
              </span>
            </div>
          ) : (
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
          )}
        </div>
      </div>

      {/* AI Copilot Modal for SQL */}
      <AiCopilotModal
        isOpen={isAiModalOpen}
        onClose={() => setIsAiModalOpen(false)}
        activeDb={activeDb}
        selectedTable={selectedTable}
        onApplySql={(generatedSql) => {
          setSqlQuery(generatedSql);
        }}
      />

      {/* Edit Connection Configuration Modal */}
      <NewConnectionModal
        isOpen={isEditModalOpen}
        initialConnection={activeDb}
        onClose={() => setIsEditModalOpen(false)}
        onConnect={(updatedDb) => {
          if (onUpdateDatabase) onUpdateDatabase(updatedDb);
          setIsEditModalOpen(false);
        }}
      />
    </div>
  );
};
