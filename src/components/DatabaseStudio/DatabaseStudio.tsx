import React, { useState, useEffect, useRef } from 'react';
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
  Settings,
  FileCode,
  Plus,
  X,
  Terminal,
  Zap,
  Copy,
  AlertTriangle
} from 'lucide-react';
import { DatabaseConnection, TableSchema, SqlScriptTab } from '../../types';
import { SqliteDropZone } from './SqliteDropZone';
import { AiCopilotModal } from '../AiCopilot/AiCopilotModal';
import { NewConnectionModal } from './NewConnectionModal';
import { DatabaseService } from '../../services/databaseService';
import { SAMPLE_PLAYGROUND_DB, SAMPLE_MONGODB_PLAYGROUND_DB } from '../../data/mockData';

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
  queryPlan?: string;
  command?: string;
}

export const DatabaseStudio: React.FC<DatabaseStudioProps> = ({
  activeDb,
  onRecordHistory,
  onDatabaseLoaded,
  onRenameDatabase,
  onUpdateDatabase
}) => {
  const safeDb = activeDb || { id: 'db-empty', name: 'No Connection', type: 'postgres', database: '', isConnected: false, tables: [] };
  const safeTables = Array.isArray(safeDb.tables) ? safeDb.tables : [];

  const [selectedTable, setSelectedTable] = useState<TableSchema | null>(
    safeTables[0] || null
  );
  const [sqlQuery, setSqlQuery] = useState<string>('');
  const [isExecuting, setIsExecuting] = useState(false);
  const [searchFilter, setSearchFilter] = useState('');
  const [isAiModalOpen, setIsAiModalOpen] = useState(false);
  const [isEditModalOpen, setIsEditModalOpen] = useState(false);

  const editorRef = useRef<HTMLTextAreaElement>(null);

  // DBeaver-style multiple script tabs state
  const [scriptTabs, setScriptTabs] = useState<SqlScriptTab[]>(() => {
    try {
      const saved = localStorage.getItem(`bapu_db_scripts_${safeDb.id}`);
      if (saved) {
        const parsed = JSON.parse(saved);
        if (Array.isArray(parsed) && parsed.length > 0) return parsed;
      }
    } catch {}
    return [{
      id: `tab-${safeDb.id}-1`,
      databaseId: safeDb.id,
      name: safeDb.type === 'mongodb' ? 'Script 1.mongodb.js' : 'Script 1.sql',
      query: ''
    }];
  });

  const [activeTabId, setActiveTabId] = useState<string>(() => scriptTabs[0]?.id || 'tab-1');
  const [editingTabId, setEditingTabId] = useState<string | null>(null);
  const [editingTabName, setEditingTabName] = useState<string>('');
  const [selectionToast, setSelectionToast] = useState<string | null>(null);
  const [activeResultView, setActiveResultView] = useState<'grid' | 'console'>('grid');
  const [copiedPlan, setCopiedPlan] = useState(false);
  const [copiedSql, setCopiedSql] = useState(false);
  const [lastExecutedSql, setLastExecutedSql] = useState<string>('');

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

  // Synchronize script tabs when active database connection changes
  useEffect(() => {
    try {
      const saved = localStorage.getItem(`bapu_db_scripts_${safeDb.id}`);
      if (saved) {
        const parsed = JSON.parse(saved);
        if (Array.isArray(parsed) && parsed.length > 0) {
          setScriptTabs(parsed);
          setActiveTabId(parsed[0].id);
          setSqlQuery(parsed[0].query || '');
          return;
        }
      }
    } catch {}

    const initialTab: SqlScriptTab = {
      id: `tab-${safeDb.id}-${Date.now()}`,
      databaseId: safeDb.id,
      name: safeDb.type === 'mongodb' ? 'Script 1.mongodb.js' : 'Script 1.sql',
      query: ''
    };
    setScriptTabs([initialTab]);
    setActiveTabId(initialTab.id);
    setSqlQuery('');
  }, [safeDb.id]);

  // Query results state
  const [queryResult, setQueryResult] = useState<QueryResult>({
    columns: [],
    rows: [],
    rowCount: 0,
    executionTimeMs: 0
  });

  // Auto-sync initial table preview when active database changes
  useEffect(() => {
    const firstTable = safeTables.find(t => !t.name.startsWith('pg_') && !t.name.startsWith('sql_')) || safeTables[0];
    if (firstTable && safeDb.id !== 'db-empty') {
      setSelectedTable(firstTable);
      const query = safeDb.type === 'mongodb' 
        ? `${firstTable.name}.find({})` 
        : `SELECT * FROM ${firstTable.name} LIMIT 15;`;

      // Set initial query on active tab
      setSqlQuery(query);
      setScriptTabs(prev => {
        const updated = prev.map(t => t.id === activeTabId ? { ...t, query } : t);
        try { localStorage.setItem(`bapu_db_scripts_${safeDb.id}`, JSON.stringify(updated)); } catch {}
        return updated;
      });

      DatabaseService.executeQuery(safeDb, query).then(res => {
        if (res.success) {
          setQueryResult({
            columns: res.columns || [],
            rows: res.rows || [],
            rowCount: res.rowCount || 0,
            executionTimeMs: res.executionTimeMs || 0,
            command: (res as any).command,
            queryPlan: (res as any).queryPlan
          });
        } else {
          setQueryResult({
            columns: [],
            rows: [],
            rowCount: 0,
            executionTimeMs: res.executionTimeMs || 0,
            error: res.message
          });
        }
      }).catch(err => {
        setQueryResult({
          columns: [],
          rows: [],
          rowCount: 0,
          executionTimeMs: 0,
          error: err?.message || 'Execution failed'
        });
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
  }, [safeDb.id, safeTables.length]);

  const handleSelectScriptTab = (tabId: string) => {
    const tab = scriptTabs.find(t => t.id === tabId);
    if (!tab) return;
    setActiveTabId(tabId);
    setSqlQuery(tab.query || '');
  };

  const handleAddScriptTab = () => {
    const ext = safeDb.type === 'mongodb' ? 'mongodb.js' : 'sql';
    const newTab: SqlScriptTab = {
      id: `tab-${safeDb.id}-${Date.now()}`,
      databaseId: safeDb.id,
      name: `Script ${scriptTabs.length + 1}.${ext}`,
      query: safeDb.type === 'mongodb' ? '// New MongoDB Query Script\n' : '-- New SQL Query Script\n'
    };
    const updated = [...scriptTabs, newTab];
    setScriptTabs(updated);
    setActiveTabId(newTab.id);
    setSqlQuery(newTab.query);
    try {
      localStorage.setItem(`bapu_db_scripts_${safeDb.id}`, JSON.stringify(updated));
    } catch {}
  };

  const handleCloseScriptTab = (tabId: string) => {
    if (scriptTabs.length <= 1) return;
    const remaining = scriptTabs.filter(t => t.id !== tabId);
    setScriptTabs(remaining);
    if (activeTabId === tabId) {
      const nextTab = remaining[0];
      setActiveTabId(nextTab.id);
      setSqlQuery(nextTab.query || '');
    }
    try {
      localStorage.setItem(`bapu_db_scripts_${safeDb.id}`, JSON.stringify(remaining));
    } catch {}
  };

  const handleRenameScriptTab = (tabId: string, newName: string) => {
    setEditingTabId(null);
    if (!newName.trim()) return;
    const updated = scriptTabs.map(t => t.id === tabId ? { ...t, name: newName.trim() } : t);
    setScriptTabs(updated);
    try {
      localStorage.setItem(`bapu_db_scripts_${safeDb.id}`, JSON.stringify(updated));
    } catch {}
  };

  const handleQueryChange = (newVal: string) => {
    setSqlQuery(newVal);
    setScriptTabs(prev => {
      const updated = prev.map(t => t.id === activeTabId ? { ...t, query: newVal } : t);
      try {
        localStorage.setItem(`bapu_db_scripts_${safeDb.id}`, JSON.stringify(updated));
      } catch {}
      return updated;
    });
  };

  const handleExecuteSql = async (overrideQuery?: string) => {
    let queryToRun = overrideQuery;

    // If no override was provided, check if user highlighted a specific block in the editor
    if (!queryToRun && editorRef.current) {
      const { selectionStart, selectionEnd, value } = editorRef.current;
      if (selectionStart !== selectionEnd) {
        const selectedText = value.substring(selectionStart, selectionEnd).trim();
        if (selectedText) {
          queryToRun = selectedText;
          setSelectionToast(`⚡ Running selection (${selectedText.length} chars)`);
          setTimeout(() => setSelectionToast(null), 2500);
        }
      }
    }

    if (!queryToRun) {
      queryToRun = sqlQuery;
    }

    if (!queryToRun || !queryToRun.trim()) return;

    setLastExecutedSql(queryToRun);
    setIsExecuting(true);
    const result = await DatabaseService.executeQuery(safeDb, queryToRun);
    setIsExecuting(false);

    if (result.success) {
      setQueryResult({
        columns: result.columns || [],
        rows: result.rows || [],
        rowCount: result.rowCount || 0,
        executionTimeMs: result.executionTimeMs || 0,
        queryPlan: (result as any).queryPlan,
        command: (result as any).command
      });
      onRecordHistory(`SQL: ${queryToRun.substring(0, 30)}...`, `${result.rowCount} rows • ${result.executionTimeMs}ms`);
    } else {
      setQueryResult(prev => ({
        ...prev,
        error: result.message,
        executionTimeMs: result.executionTimeMs || 0
      }));
      onRecordHistory(`SQL Error: ${queryToRun.substring(0, 25)}`, result.message || 'Execution error');
      setActiveResultView('console');
    }
  };

  const handleExplainQuery = () => {
    let queryToExplain = sqlQuery;
    if (editorRef.current) {
      const { selectionStart, selectionEnd, value } = editorRef.current;
      if (selectionStart !== selectionEnd) {
        const selectedText = value.substring(selectionStart, selectionEnd).trim();
        if (selectedText) queryToExplain = selectedText;
      }
    }
    if (!queryToExplain.trim()) return;

    const wrapped = DatabaseService.wrapExplainQuery(safeDb.type, queryToExplain);
    setSqlQuery(wrapped);
    handleQueryChange(wrapped);
    setActiveResultView('console');
    handleExecuteSql(wrapped);
  };

  const handleExportCsv = () => {
    if (!queryResult.rows || queryResult.rows.length === 0) return;
    const header = (queryResult.columns || []).join(',');
    const rows = queryResult.rows.map(r => (queryResult.columns || []).map(col => JSON.stringify(r[col] || '')).join(','));
    const csvContent = 'data:text/csv;charset=utf-8,' + [header, ...rows].join('\n');
    const encodedUri = encodeURI(csvContent);
    const link = document.createElement('a');
    link.setAttribute('href', encodedUri);
    link.setAttribute('download', `${(safeDb.name || 'db').toLowerCase().replace(/\s+/g, '_')}_export.csv`);
    document.body.appendChild(link);
    link.click();
    document.body.removeChild(link);
  };

  const filteredRows = (queryResult.rows || []).filter(row => {
    if (!searchFilter) return true;
    return Object.values(row || {}).some(val => 
      String(val ?? '').toLowerCase().includes(searchFilter.toLowerCase())
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
            value={safeDb.name || ''}
            onChange={(e) => {
              if (onRenameDatabase) onRenameDatabase(safeDb.id, e.target.value);
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

        <div className="sidebar-section-header">Tables ({safeTables.length})</div>
        {safeTables.length === 0 ? (
          <div style={{ fontSize: '11px', color: 'var(--text-dim)', padding: '16px 8px', lineHeight: 1.4, textAlign: 'center' }}>
            <p style={{ marginBottom: '10px' }}>
              {safeDb.id === 'db-empty' ? 'No database connection selected.' : 'No tables discovered.'}
            </p>
            {onDatabaseLoaded && (
              <div style={{ display: 'flex', flexDirection: 'column', gap: '6px' }}>
                <button
                  onClick={() => onDatabaseLoaded(SAMPLE_PLAYGROUND_DB)}
                  className="btn-secondary"
                  style={{ fontSize: '11px', padding: '6px 10px', color: '#10b981', borderColor: 'rgba(16, 185, 129, 0.3)', width: '100%' }}
                >
                  ⚡ Load SQL Playground
                </button>
                <button
                  onClick={() => onDatabaseLoaded(SAMPLE_MONGODB_PLAYGROUND_DB)}
                  className="btn-secondary"
                  style={{ fontSize: '11px', padding: '6px 10px', color: '#06b6d4', borderColor: 'rgba(6, 182, 212, 0.3)', width: '100%' }}
                >
                  🍃 Load MongoDB Playground
                </button>
              </div>
            )}
          </div>
        ) : (
          safeTables.map(table => (
            <div key={table?.name || Math.random().toString()} style={{ marginBottom: '8px' }}>
              <div
                onClick={() => {
                  setSelectedTable(table);
                  const query = safeDb.type === 'mongodb' 
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
                  {(table.rowCount ?? 0).toLocaleString()}
                </span>
              </div>

              {selectedTable?.name === table.name && (
                <div style={{ paddingLeft: '18px', marginTop: '4px' }}>
                  {(table.columns || []).map(col => (
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
        {/* Top Query Editor Box with DBeaver-Style Multi-Script Tabs */}
        <div className="sql-editor-box">
          {/* Script Tabs Bar */}
          <div style={{
            display: 'flex',
            alignItems: 'center',
            background: 'var(--bg-card)',
            borderBottom: '1px solid var(--border-subtle)',
            padding: '0 8px',
            gap: '4px',
            overflowX: 'auto',
            minHeight: '34px'
          }}>
            {scriptTabs.map(tab => {
              const isActive = tab.id === activeTabId;
              const isEditing = editingTabId === tab.id;
              return (
                <div
                  key={tab.id}
                  onClick={() => handleSelectScriptTab(tab.id)}
                  onDoubleClick={() => {
                    setEditingTabId(tab.id);
                    setEditingTabName(tab.name);
                  }}
                  style={{
                    display: 'flex',
                    alignItems: 'center',
                    gap: '6px',
                    padding: '6px 10px',
                    borderBottom: isActive ? '2px solid #10b981' : '2px solid transparent',
                    background: isActive ? 'var(--bg-input)' : 'transparent',
                    color: isActive ? '#fff' : 'var(--text-dim)',
                    fontSize: '11px',
                    cursor: 'pointer',
                    userSelect: 'none',
                    whiteSpace: 'nowrap',
                    borderRadius: 'var(--radius-sm) var(--radius-sm) 0 0',
                    transition: 'all 0.15s ease'
                  }}
                  title="Click to switch script • Double-click to rename"
                >
                  <FileCode size={12} color={isActive ? '#10b981' : 'var(--text-dim)'} />
                  {isEditing ? (
                    <input
                      type="text"
                      value={editingTabName}
                      autoFocus
                      onChange={(e) => setEditingTabName(e.target.value)}
                      onBlur={() => handleRenameScriptTab(tab.id, editingTabName)}
                      onKeyDown={(e) => {
                        if (e.key === 'Enter') handleRenameScriptTab(tab.id, editingTabName);
                        if (e.key === 'Escape') setEditingTabId(null);
                      }}
                      onClick={(e) => e.stopPropagation()}
                      style={{
                        background: 'var(--bg-main)',
                        border: '1px solid var(--border-subtle)',
                        color: '#fff',
                        fontSize: '11px',
                        padding: '1px 4px',
                        borderRadius: '2px',
                        outline: 'none'
                      }}
                    />
                  ) : (
                    <span>{tab.name}</span>
                  )}

                  {scriptTabs.length > 1 && (
                    <button
                      type="button"
                      onClick={(e) => {
                        e.stopPropagation();
                        handleCloseScriptTab(tab.id);
                      }}
                      style={{
                        background: 'none',
                        border: 'none',
                        color: 'var(--text-dim)',
                        cursor: 'pointer',
                        padding: '1px',
                        display: 'flex',
                        alignItems: 'center',
                        borderRadius: '2px'
                      }}
                      title="Close script tab"
                    >
                      <X size={11} />
                    </button>
                  )}
                </div>
              );
            })}

            <button
              type="button"
              onClick={handleAddScriptTab}
              style={{
                background: 'none',
                border: 'none',
                color: 'var(--text-dim)',
                cursor: 'pointer',
                display: 'flex',
                alignItems: 'center',
                gap: '4px',
                padding: '4px 8px',
                fontSize: '11px',
                borderRadius: 'var(--radius-sm)'
              }}
              title="Add new script scratchpad"
            >
              <Plus size={12} />
              <span>New Script</span>
            </button>

            {selectionToast && (
              <div style={{
                marginLeft: 'auto',
                fontSize: '10px',
                color: '#10b981',
                background: 'rgba(16, 185, 129, 0.12)',
                border: '1px solid rgba(16, 185, 129, 0.3)',
                borderRadius: 'var(--radius-sm)',
                padding: '2px 8px'
              }}>
                {selectionToast}
              </div>
            )}
          </div>

          <textarea
            ref={editorRef}
            value={sqlQuery}
            onChange={(e) => handleQueryChange(e.target.value)}
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
              ? 'Write MongoDB JSON query or MQL (e.g. { "status": "active" } or users.find({})) • Highlight query & press Ctrl+Enter to run selection'
              : 'Write SQL query here... (e.g. SELECT * FROM table;) • Highlight query & press Ctrl+Enter to run selection'}
          />

          <div className="sql-actions-bar" style={{ flexWrap: 'wrap', gap: '6px' }}>
            <div style={{ display: 'flex', gap: '6px', flexWrap: 'wrap', alignItems: 'center' }}>
              {selectedTable && (
                safeDb.type === 'mongodb' ? (
                  <>
                    <button 
                      onClick={() => {
                        const q = `${selectedTable.name}.find({})`;
                        handleQueryChange(q);
                        handleExecuteSql(q);
                      }}
                      className="btn-secondary"
                      style={{ fontSize: '11px', padding: '3px 8px' }}
                    >
                      Find All ({selectedTable.name})
                    </button>
                    <button 
                      onClick={() => {
                        const cols = selectedTable.columns || [];
                        const catCol = cols.find(c => /status|role|type|category|plan/i.test(c.name))?.name || 'status';
                        const q = `${selectedTable.name}.aggregate([\n  { $group: { _id: "$${catCol}", count: { $sum: 1 } } },\n  { $sort: { count: -1 } }\n])`;
                        handleQueryChange(q);
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
                        const cols = selectedTable.columns || [];
                        const dateCol = cols.find(c => /date|time|created|updated|at$/i.test(c.name))?.name || (cols.find(c => c.isPrimaryKey)?.name || 'id');
                        const q = `SELECT * FROM ${selectedTable.name} ORDER BY ${dateCol} DESC LIMIT 25;`;
                        handleQueryChange(q);
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
                        const cols = selectedTable.columns || [];
                        const catCol = cols.find(c => /status|role|type|category|plan|state/i.test(c.name))?.name || (cols.length > 1 ? cols[1].name : 'id');
                        const q = `SELECT ${catCol}, COUNT(*) as total FROM ${selectedTable.name} GROUP BY ${catCol} ORDER BY total DESC;`;
                        handleQueryChange(q);
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
                        const cols = selectedTable.columns || [];
                        const nullCol = cols.find(c => !c.isPrimaryKey)?.name || 'id';
                        const q = `SELECT * FROM ${selectedTable.name} WHERE ${nullCol} IS NULL;`;
                        handleQueryChange(q);
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
                        const cols = selectedTable.columns || [];
                        const colStr = cols.length > 0 ? cols.map(c => c.name).join(', ') : '*';
                        const q = `SELECT ${colStr} FROM ${selectedTable.name} LIMIT 50;`;
                        handleQueryChange(q);
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
                type="button"
                onClick={handleExplainQuery}
                disabled={isExecuting}
                className="btn-secondary"
                style={{ borderColor: 'rgba(245, 158, 11, 0.4)', color: '#fbbf24', padding: '6px 12px', fontSize: '12px' }}
                title="Explain Query Execution Plan (Costs, Scans, Buffers)"
              >
                <Zap size={13} color="#f59e0b" />
                <span>Explain Plan</span>
              </button>

              <button 
                onClick={() => handleExecuteSql()}
                disabled={isExecuting}
                className="btn-send"
                style={{ padding: '6px 14px', fontSize: '12px' }}
                title="Run Query or Selection (Ctrl+Enter / F5)"
              >
                <Play size={13} />
                <span>{isExecuting ? 'Running...' : 'Run Query (F5)'}</span>
              </button>
            </div>
          </div>
        </div>

        {/* Results Toolbar with Dual View Tabs (Data Grid vs Execution Console) */}
        <div className="panel-tab-header">
          <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
            <div className="panel-tabs">
              <button
                className={`subtab-btn ${activeResultView === 'grid' ? 'active' : ''}`}
                onClick={() => setActiveResultView('grid')}
                style={{ display: 'flex', alignItems: 'center', gap: '6px' }}
              >
                <Table size={13} />
                <span>Data Grid ({queryResult.rowCount})</span>
              </button>
              <button
                className={`subtab-btn ${activeResultView === 'console' ? 'active' : ''}`}
                onClick={() => setActiveResultView('console')}
                style={{ display: 'flex', alignItems: 'center', gap: '6px' }}
              >
                <Terminal size={13} />
                <span>Execution Console</span>
                {queryResult.error ? (
                  <span style={{ width: '6px', height: '6px', borderRadius: '50%', background: '#ef4444' }} />
                ) : queryResult.rowCount > 0 ? (
                  <span style={{ width: '6px', height: '6px', borderRadius: '50%', background: '#10b981' }} />
                ) : null}
              </button>
            </div>

            <span style={{ fontSize: '11px', color: 'var(--text-muted)', fontFamily: 'var(--font-mono)', marginLeft: '6px' }}>
              {queryResult.rowCount} rows • {queryResult.executionTimeMs}ms
            </span>
          </div>

          {activeResultView === 'grid' && (
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
          )}
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
            fontFamily: 'var(--font-mono)',
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'space-between'
          }}>
            <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
              <AlertTriangle size={15} color="#ef4444" />
              <span>{queryResult.error}</span>
            </div>
            <button
              onClick={() => setActiveResultView('console')}
              className="btn-secondary"
              style={{ fontSize: '10px', padding: '2px 8px', borderColor: 'rgba(239, 68, 68, 0.4)', color: '#fca5a5' }}
            >
              View Console Details
            </button>
          </div>
        )}

        {/* Dual Content View: Data Grid or Execution Console */}
        {activeResultView === 'grid' ? (
          <div className="data-grid-container">
            {queryResult.columns.length === 0 ? (
              <div style={{ display: 'flex', flexDirection: 'column', alignItems: 'center', justifyContent: 'center', height: '100%', color: 'var(--text-dim)', gap: '8px', padding: '40px' }}>
                <Database size={24} style={{ opacity: 0.3 }} />
                <span style={{ fontSize: '12px' }}>
                  {safeDb.id === 'db-empty'
                    ? 'No database connection connected. Add a connection in the sidebar or drop a SQLite file.'
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
                          {row[col] === null || row[col] === undefined ? (
                            <span style={{ opacity: 0.5, fontStyle: 'italic' }}>NULL</span>
                          ) : typeof row[col] === 'object' ? (
                            JSON.stringify(row[col])
                          ) : (
                            String(row[col])
                          )}
                        </td>
                      ))}
                    </tr>
                  ))}
                </tbody>
              </table>
            )}
          </div>
        ) : (
          /* Execution Console & Server Output View */
          <div style={{ flex: 1, overflowY: 'auto', padding: '16px', background: '#080c13', display: 'flex', flexDirection: 'column', gap: '16px' }}>
            {/* Metadata Stats Grid */}
            <div style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fit, minmax(140px, 1fr))', gap: '10px' }}>
              <div style={{ background: 'var(--bg-card)', border: '1px solid var(--border-subtle)', borderRadius: 'var(--radius-md)', padding: '10px 14px' }}>
                <div style={{ fontSize: '10px', color: 'var(--text-dim)', textTransform: 'uppercase', letterSpacing: '0.05em' }}>Status</div>
                <div style={{ fontSize: '13px', fontWeight: 600, color: queryResult.error ? '#ef4444' : '#10b981', marginTop: '4px' }}>
                  {queryResult.error ? 'FAILED' : 'SUCCESS (200 OK)'}
                </div>
              </div>

              <div style={{ background: 'var(--bg-card)', border: '1px solid var(--border-subtle)', borderRadius: 'var(--radius-md)', padding: '10px 14px' }}>
                <div style={{ fontSize: '10px', color: 'var(--text-dim)', textTransform: 'uppercase', letterSpacing: '0.05em' }}>Execution Time</div>
                <div style={{ fontSize: '13px', fontWeight: 600, color: '#60a5fa', marginTop: '4px', fontFamily: 'var(--font-mono)' }}>
                  {queryResult.executionTimeMs} ms
                </div>
              </div>

              <div style={{ background: 'var(--bg-card)', border: '1px solid var(--border-subtle)', borderRadius: 'var(--radius-md)', padding: '10px 14px' }}>
                <div style={{ fontSize: '10px', color: 'var(--text-dim)', textTransform: 'uppercase', letterSpacing: '0.05em' }}>Rows Affected</div>
                <div style={{ fontSize: '13px', fontWeight: 600, color: '#f59e0b', marginTop: '4px', fontFamily: 'var(--font-mono)' }}>
                  {queryResult.rowCount.toLocaleString()}
                </div>
              </div>

              <div style={{ background: 'var(--bg-card)', border: '1px solid var(--border-subtle)', borderRadius: 'var(--radius-md)', padding: '10px 14px' }}>
                <div style={{ fontSize: '10px', color: 'var(--text-dim)', textTransform: 'uppercase', letterSpacing: '0.05em' }}>Driver / Command</div>
                <div style={{ fontSize: '13px', fontWeight: 600, color: '#a78bfa', marginTop: '4px', fontFamily: 'var(--font-mono)' }}>
                  {safeDb.type.toUpperCase()} • {queryResult.command || (safeDb.type === 'mongodb' ? 'MQL' : 'QUERY')}
                </div>
              </div>
            </div>

            {/* Execution Plan Output (if available) */}
            {queryResult.queryPlan && (
              <div style={{ background: 'var(--bg-card)', border: '1px solid rgba(245, 158, 11, 0.3)', borderRadius: 'var(--radius-md)', overflow: 'hidden' }}>
                <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', background: 'rgba(245, 158, 11, 0.08)', padding: '8px 12px', borderBottom: '1px solid rgba(245, 158, 11, 0.2)' }}>
                  <div style={{ display: 'flex', alignItems: 'center', gap: '6px', fontSize: '11px', fontWeight: 600, color: '#fbbf24' }}>
                    <Zap size={13} />
                    <span>Query Execution Plan (Explain Analyzer)</span>
                  </div>
                  <button
                    onClick={() => {
                      navigator.clipboard.writeText(queryResult.queryPlan || '');
                      setCopiedPlan(true);
                      setTimeout(() => setCopiedPlan(false), 2000);
                    }}
                    className="btn-secondary"
                    style={{ fontSize: '10px', padding: '2px 8px' }}
                  >
                    {copiedPlan ? <Check size={11} color="#10b981" /> : <Copy size={11} />}
                    <span>{copiedPlan ? 'Copied' : 'Copy Plan'}</span>
                  </button>
                </div>
                <pre style={{ margin: 0, padding: '14px', fontFamily: 'var(--font-mono)', fontSize: '11px', color: '#e2e8f0', lineHeight: 1.5, overflowX: 'auto', background: '#05080e' }}>
                  {queryResult.queryPlan}
                </pre>
              </div>
            )}

            {/* Executed Statement Block */}
            <div style={{ background: 'var(--bg-card)', border: '1px solid var(--border-subtle)', borderRadius: 'var(--radius-md)', overflow: 'hidden' }}>
              <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', background: 'rgba(15, 21, 34, 0.7)', padding: '8px 12px', borderBottom: '1px solid var(--border-subtle)' }}>
                <span style={{ fontSize: '11px', fontWeight: 600, color: 'var(--text-muted)' }}>Executed SQL / Command</span>
                <button
                  onClick={() => {
                    navigator.clipboard.writeText(lastExecutedSql || sqlQuery);
                    setCopiedSql(true);
                    setTimeout(() => setCopiedSql(false), 2000);
                  }}
                  className="btn-secondary"
                  style={{ fontSize: '10px', padding: '2px 8px' }}
                >
                  {copiedSql ? <Check size={11} color="#10b981" /> : <Copy size={11} />}
                  <span>{copiedSql ? 'Copied' : 'Copy Query'}</span>
                </button>
              </div>
              <pre style={{ margin: 0, padding: '14px', fontFamily: 'var(--font-mono)', fontSize: '11px', color: '#60a5fa', lineHeight: 1.5, overflowX: 'auto', background: '#05080e' }}>
                {lastExecutedSql || sqlQuery || '-- No query executed yet'}
              </pre>
            </div>
          </div>
        )}
      </div>

      {/* AI Copilot Modal for SQL */}
      <AiCopilotModal
        isOpen={isAiModalOpen}
        onClose={() => setIsAiModalOpen(false)}
        activeDb={safeDb}
        selectedTable={selectedTable}
        onApplySql={(generatedSql) => {
          setSqlQuery(generatedSql);
        }}
      />

      {/* Edit Connection Configuration Modal */}
      <NewConnectionModal
        isOpen={isEditModalOpen}
        initialConnection={safeDb}
        onClose={() => setIsEditModalOpen(false)}
        onConnect={(updatedDb) => {
          if (onUpdateDatabase) onUpdateDatabase(updatedDb);
          setIsEditModalOpen(false);
        }}
      />
    </div>
  );
};
