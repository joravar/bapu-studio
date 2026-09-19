import React, { useState, useEffect } from 'react';
import { Header } from './components/Header';
import { Sidebar, WorkspaceTab } from './components/Sidebar';
import { ApiStudio } from './components/ApiStudio/ApiStudio';
import { DatabaseStudio } from './components/DatabaseStudio/DatabaseStudio';
import { StreamStudio } from './components/StreamStudio/StreamStudio';
import { SecretsStudio } from './components/SecretsStudio/SecretsStudio';
import { HistoryStudio } from './components/HistoryStudio/HistoryStudio';
import { ErrorBoundary } from './components/ErrorBoundary';
import { 
  INITIAL_COLLECTIONS, 
  INITIAL_DATABASES, 
  INITIAL_ENVIRONMENTS, 
  INITIAL_HISTORY 
} from './data/mockData';
import { ApiRequest, Collection, DatabaseConnection, Environment, HistoryItem } from './types';
import { Globe, Database, KeyRound, Radio, Sparkles, X, Plus } from 'lucide-react';
import { DatabaseService } from './services/databaseService';
import { CollectionSyncService } from './services/collectionSyncService';
import { secureGetItem, secureSetItem } from './utils/secureStorage';
import { applyTheme, getStoredTheme, Theme } from './utils/theme';

export function sanitizeDatabase(db: any): DatabaseConnection {
  if (!db || typeof db !== 'object') {
    return { id: 'db-empty', name: 'No Connection', type: 'postgres', database: '', isConnected: false, tables: [] };
  }
  return {
    id: String(db.id || `db-${Date.now()}`),
    name: String(db.name || 'Database Connection'),
    type: db.type || 'postgres',
    database: String(db.database || ''),
    connectionString: db.connectionString ? String(db.connectionString) : undefined,
    host: db.host ? String(db.host) : undefined,
    port: db.port ? String(db.port) : undefined,
    username: db.username ? String(db.username) : undefined,
    password: db.password ? String(db.password) : undefined,
    ssl: Boolean(db.ssl),
    sslCaCert: db.sslCaCert ? String(db.sslCaCert) : undefined,
    sslClientCert: db.sslClientCert ? String(db.sslClientCert) : undefined,
    sslClientKey: db.sslClientKey ? String(db.sslClientKey) : undefined,
    sslRejectUnauthorized: db.sslRejectUnauthorized !== undefined ? Boolean(db.sslRejectUnauthorized) : undefined,
    sshEnabled: Boolean(db.sshEnabled),
    sshHost: db.sshHost ? String(db.sshHost) : undefined,
    sshPort: db.sshPort ? String(db.sshPort) : undefined,
    sshUsername: db.sshUsername ? String(db.sshUsername) : undefined,
    sshPassword: db.sshPassword ? String(db.sshPassword) : undefined,
    sshPrivateKey: db.sshPrivateKey ? String(db.sshPrivateKey) : undefined,
    sshPassphrase: db.sshPassphrase ? String(db.sshPassphrase) : undefined,
    isConnected: Boolean(db.isConnected),
    isDemoDb: Boolean(db.isDemoDb),
    tables: Array.isArray(db.tables) ? db.tables.map((t: any) => ({
      name: String(t?.name || 'table'),
      rowCount: typeof t?.rowCount === 'number' ? t.rowCount : 0,
      columns: Array.isArray(t?.columns) ? t.columns.map((c: any) => ({
        name: String(c?.name || 'col'),
        type: String(c?.type || 'VARCHAR'),
        isPrimaryKey: Boolean(c?.isPrimaryKey),
        isNullable: Boolean(c?.isNullable)
      })) : []
    })) : []
  };
}

export const App: React.FC = () => {
  // Navigation & Workspace State with LocalStorage Persistence
  const [activeTab, setActiveTab] = useState<WorkspaceTab>('api');

  const [theme, setTheme] = useState<Theme>(() => getStoredTheme());
  useEffect(() => { applyTheme(theme); }, [theme]);
  const handleToggleTheme = () => setTheme(prev => (prev === 'dark' ? 'light' : 'dark'));

  const [collections, setCollections] = useState<Collection[]>(() => {
    try {
      const saved = secureGetItem('bapu_collections');
      if (saved) return JSON.parse(saved);
    } catch {}
    return INITIAL_COLLECTIONS;
  });

  const [activeRequest, setActiveRequest] = useState<ApiRequest | null>(() => {
    try {
      const saved = secureGetItem('bapu_collections');
      if (saved) {
        const parsed = JSON.parse(saved);
        if (parsed[0]?.requests[0]) return parsed[0].requests[0];
      }
    } catch {}
    return null;
  });

  const [databases, setDatabases] = useState<DatabaseConnection[]>(() => {
    try {
      const saved = secureGetItem('bapu_databases');
      if (saved) {
        const parsed = JSON.parse(saved);
        if (Array.isArray(parsed)) {
          return parsed.map(sanitizeDatabase);
        }
      }
    } catch {}
    return INITIAL_DATABASES.map(sanitizeDatabase);
  });

  const [activeDb, setActiveDb] = useState<DatabaseConnection>(() => {
    try {
      const saved = secureGetItem('bapu_databases');
      if (saved) {
        const parsed = JSON.parse(saved);
        if (Array.isArray(parsed) && parsed[0]) return sanitizeDatabase(parsed[0]);
      }
    } catch {}
    return sanitizeDatabase(INITIAL_DATABASES[0]);
  });

  const [environments, setEnvironments] = useState<Environment[]>(() => {
    try {
      const saved = secureGetItem('bapu_environments');
      if (saved) return JSON.parse(saved);
    } catch {}
    return INITIAL_ENVIRONMENTS;
  });

  const [activeEnv, setActiveEnv] = useState<Environment>(() => {
    try {
      const saved = secureGetItem('bapu_environments');
      if (saved) {
        const parsed = JSON.parse(saved);
        if (parsed[0]) return parsed[0];
      }
    } catch {}
    return INITIAL_ENVIRONMENTS[0];
  });

  const [history, setHistory] = useState<HistoryItem[]>(() => {
    try {
      const saved = localStorage.getItem('bapu_history');
      if (saved) return JSON.parse(saved);
    } catch {}
    return INITIAL_HISTORY;
  });


  // Automatically save any changes to localStorage (secret-bearing state is encrypted at rest — see secureStorage.ts)
  useEffect(() => {
    try {
      secureSetItem('bapu_collections', JSON.stringify(collections));
    } catch {}
  }, [collections]);

  // Mirror any folder-linked collection to disk as plain JSON files (one per request), debounced
  // so rapid edits (e.g. typing in a request body) don't hammer the filesystem on every keystroke.
  useEffect(() => {
    const linked = collections.filter(c => c.folderPath);
    if (linked.length === 0) return;

    const timer = setTimeout(() => {
      linked.forEach(col => {
        CollectionSyncService.writeToFolder(col.folderPath!, col.name, col.requests).catch(() => {});
      });
    }, 700);

    return () => clearTimeout(timer);
  }, [collections]);

  useEffect(() => {
    try {
      secureSetItem('bapu_databases', JSON.stringify(databases));
    } catch {}
  }, [databases]);

  useEffect(() => {
    try {
      secureSetItem('bapu_environments', JSON.stringify(environments));
    } catch {}
  }, [environments]);

  useEffect(() => {
    try {
      localStorage.setItem('bapu_history', JSON.stringify(history));
    } catch {}
  }, [history]);

  // Handlers for Request updates
  const handleUpdateRequest = (updated: ApiRequest) => {
    setActiveRequest(updated);
    setCollections(prev => prev.map(col => ({
      ...col,
      requests: col.requests.map(req => req.id === updated.id ? updated : req)
    })));
  };

  const handleAddCollection = (name: string) => {
    const newCol: Collection = {
      id: `col-${Date.now()}`,
      name,
      requests: []
    };
    setCollections(prev => [...prev, newCol]);
    handleNewRequestInCollection(newCol.id);
  };

  const handleDeleteCollection = (collectionId: string) => {
    setCollections(prev => prev.filter(c => c.id !== collectionId));
  };

  const handleNewRequestInCollection = (collectionId: string) => {
    const newReq: ApiRequest = {
      id: `req-${Date.now()}`,
      name: 'New HTTP Request',
      method: 'GET',
      url: '',
      params: [],
      headers: [{ id: 'h-def', key: 'Accept', value: 'application/json', enabled: true }],
      bodyType: 'none',
      bodyContent: '',
      authType: 'none',
      authConfig: {},
      collectionId
    };

    setCollections(prev => prev.map(col => {
      if (col.id === collectionId) {
        return { ...col, requests: [...col.requests, newReq] };
      }
      return col;
    }));

    setActiveRequest(newReq);
    setActiveTab('api');
  };

  const handleDeleteRequest = (requestId: string) => {
    let nextReq: ApiRequest | null = null;

    setCollections(prev => {
      const updated = prev.map(col => ({
        ...col,
        requests: col.requests.filter(r => r.id !== requestId)
      }));

      // Find another request to activate if the deleted one was active
      for (const col of updated) {
        if (col.requests.length > 0) {
          nextReq = col.requests[0];
          break;
        }
      }
      return updated;
    });

    if (activeRequest?.id === requestId) {
      setActiveRequest(nextReq);
    }
  };

  const handleNewRequest = () => {
    if (collections.length === 0) {
      handleAddCollection('Default Collection');
      return;
    }
    handleNewRequestInCollection(collections[0].id);
  };

  const handleAddDatabase = (newDb: DatabaseConnection) => {
    const cleanDb = sanitizeDatabase(newDb);
    setDatabases(prev => {
      const exists = prev.some(d => d.id === cleanDb.id);
      if (exists) {
        return prev.map(d => d.id === cleanDb.id ? cleanDb : d);
      }
      return [cleanDb, ...prev];
    });
    setActiveDb(cleanDb);
    setActiveTab('db');
    handleRecordHistory(`Connected: ${cleanDb.name}`, `${cleanDb.type.toUpperCase()} • ${cleanDb.database}`);
  };

  const handleUpdateDatabase = (updatedDb: DatabaseConnection) => {
    const cleanDb = sanitizeDatabase(updatedDb);
    // Drop any cached connection pool for this id so edited credentials/host take effect immediately,
    // instead of the next query silently reusing a stale pool opened with the old config.
    DatabaseService.disconnect(cleanDb.id).catch(() => {});
    setDatabases(prev => prev.map(d => d.id === cleanDb.id ? cleanDb : d));
    if (activeDb?.id === cleanDb.id) {
      setActiveDb(cleanDb);
    }
    handleRecordHistory(`Updated DB: ${cleanDb.name}`, `${cleanDb.type.toUpperCase()} • ${cleanDb.database}`);
  };

  const handleClearHistory = () => {
    setHistory([]);
    try { localStorage.removeItem('bapu_history'); } catch {}
  };

  const handleDeleteHistoryItem = (id: string) => {
    setHistory(prev => prev.filter(item => item.id !== id));
  };

  const handleReplayHistoryItem = (item: HistoryItem) => {
    if (item.title.startsWith('SQL:') || item.type === 'sql') {
      setActiveTab('db');
    } else {
      setActiveTab('api');
    }
  };

  const handleDeleteDatabase = (dbId: string) => {
    DatabaseService.disconnect(dbId).catch(() => {});
    setDatabases(prev => {
      const remaining = prev.filter(d => d.id !== dbId);
      if (activeDb?.id === dbId) {
        if (remaining.length > 0) {
          setActiveDb(remaining[0]);
        } else {
          setActiveDb({ id: 'db-empty', name: 'No Connection', type: 'postgres', database: '', isConnected: false, tables: [] });
        }
      }
      return remaining;
    });
  };

  const handleReorderCollections = (sourceIndex: number, destIndex: number) => {
    if (sourceIndex === destIndex) return;
    setCollections(prev => {
      const copy = [...prev];
      const [removed] = copy.splice(sourceIndex, 1);
      copy.splice(destIndex, 0, removed);
      return copy;
    });
  };

  const handleReorderRequests = (collectionId: string, sourceIndex: number, destIndex: number) => {
    if (sourceIndex === destIndex) return;
    setCollections(prev => prev.map(col => {
      if (col.id !== collectionId) return col;
      const copy = [...col.requests];
      const [removed] = copy.splice(sourceIndex, 1);
      copy.splice(destIndex, 0, removed);
      return { ...col, requests: copy };
    }));
  };

  const handleMoveRequest = (sourceColId: string, destColId: string, sourceIndex: number, destIndex: number) => {
    setCollections(prev => {
      const sourceCol = prev.find(c => c.id === sourceColId);
      const destCol = prev.find(c => c.id === destColId);
      if (!sourceCol || !destCol) return prev;

      const sourceRequests = [...sourceCol.requests];
      const [movedReq] = sourceRequests.splice(sourceIndex, 1);
      if (!movedReq) return prev;

      movedReq.collectionId = destColId;

      if (sourceColId === destColId) {
        sourceRequests.splice(destIndex, 0, movedReq);
        return prev.map(c => c.id === sourceColId ? { ...c, requests: sourceRequests } : c);
      }

      const destRequests = [...destCol.requests];
      destRequests.splice(destIndex, 0, movedReq);

      return prev.map(c => {
        if (c.id === sourceColId) return { ...c, requests: sourceRequests };
        if (c.id === destColId) return { ...c, requests: destRequests };
        return c;
      });
    });
  };

  const handleReorderDatabases = (sourceIndex: number, destIndex: number) => {
    if (sourceIndex === destIndex) return;
    setDatabases(prev => {
      const copy = [...prev];
      const [removed] = copy.splice(sourceIndex, 1);
      copy.splice(destIndex, 0, removed);
      return copy;
    });
  };

  const handleRecordHistory = (title: string, subtitle: string, status?: number) => {
    const newItem: HistoryItem = {
      id: `hist-${Date.now()}`,
      type: activeTab === 'db' ? 'sql' : 'api',
      title,
      subtitle,
      status,
      timestamp: 'Just now'
    };
    setHistory(prev => [newItem, ...prev.slice(0, 20)]);
  };

  // Resizable sidebar state
  const [sidebarWidth, setSidebarWidth] = useState<number>(() => {
    try {
      const saved = localStorage.getItem('bapu_sidebar_width');
      if (saved) return Math.max(180, Math.min(500, Number(saved)));
    } catch {}
    return 260;
  });
  const [isResizingSidebar, setIsResizingSidebar] = useState(false);

  useEffect(() => {
    if (!isResizingSidebar) return;

    const handleMouseMove = (e: MouseEvent) => {
      const newWidth = Math.max(180, Math.min(520, e.clientX));
      setSidebarWidth(newWidth);
      try {
        localStorage.setItem('bapu_sidebar_width', String(newWidth));
      } catch {}
    };

    const handleMouseUp = () => {
      setIsResizingSidebar(false);
    };

    document.addEventListener('mousemove', handleMouseMove);
    document.addEventListener('mouseup', handleMouseUp);
    return () => {
      document.removeEventListener('mousemove', handleMouseMove);
      document.removeEventListener('mouseup', handleMouseUp);
    };
  }, [isResizingSidebar]);

  const handleRenameCollection = (collectionId: string, newName: string) => {
    setCollections(prev => prev.map(c => c.id === collectionId ? { ...c, name: newName } : c));
  };

  const handleRenameRequest = (requestId: string, newName: string) => {
    setCollections(prev => prev.map(col => ({
      ...col,
      requests: col.requests.map(r => r.id === requestId ? { ...r, name: newName } : r)
    })));
    if (activeRequest && activeRequest.id === requestId) {
      setActiveRequest(prev => prev ? { ...prev, name: newName } : null);
    }
  };

  const handleRenameDatabase = (dbId: string, newName: string) => {
    setDatabases(prev => prev.map(db => db.id === dbId ? { ...db, name: newName } : db));
    if (activeDb && activeDb.id === dbId) {
      setActiveDb(prev => prev ? { ...prev, name: newName } : prev);
    }
  };

  const handleAddEnvironment = (name: string) => {
    const newEnv: Environment = {
      id: `env-${Date.now()}`,
      name,
      variables: [
        { id: `var-${Date.now()}-1`, key: 'API_BASE_URL', value: 'https://api.example.com', enabled: true, isSecret: false }
      ]
    };
    setEnvironments(prev => [...prev, newEnv]);
    setActiveEnv(newEnv);
    handleRecordHistory(`Created Environment: ${name}`, '1 default variable');
  };

  const handleDeleteEnvironment = (envId: string) => {
    if (environments.length <= 1) return;
    const remaining = environments.filter(e => e.id !== envId);
    setEnvironments(remaining);
    if (activeEnv.id === envId) {
      setActiveEnv(remaining[0]);
    }
  };

  const handleRenameEnvironment = (envId: string, newName: string) => {
    setEnvironments(prev => prev.map(e => e.id === envId ? { ...e, name: newName } : e));
    if (activeEnv.id === envId) {
      setActiveEnv(prev => ({ ...prev, name: newName }));
    }
  };

  const handleImportCollection = (newCol: Collection) => {
    setCollections(prev => [...prev, newCol]);
    if (newCol.requests.length > 0) {
      setActiveRequest(newCol.requests[0]);
    }
    setActiveTab('api');
    handleRecordHistory(`Imported: ${newCol.name}`, `${newCol.requests.length} API endpoints parsed`);
  };

  const handleLinkCollectionFolder = async (collectionId: string) => {
    const folderPath = await CollectionSyncService.chooseFolder();
    if (!folderPath) return;

    const col = collections.find(c => c.id === collectionId);
    if (!col) return;

    setCollections(prev => prev.map(c => c.id === collectionId ? { ...c, folderPath } : c));
    const res = await CollectionSyncService.writeToFolder(folderPath, col.name, col.requests);
    handleRecordHistory(
      `Linked to Folder: ${col.name}`,
      res.success ? folderPath : `Failed: ${res.message}`
    );
  };

  const handleUnlinkCollectionFolder = (collectionId: string) => {
    setCollections(prev => prev.map(c => c.id === collectionId ? { ...c, folderPath: undefined } : c));
  };

  const handleLoadCollectionFromFolder = async () => {
    const folderPath = await CollectionSyncService.chooseFolder();
    if (!folderPath) return;

    const res = await CollectionSyncService.readFromFolder(folderPath);
    if (!res.success || !res.requests) {
      handleRecordHistory('Load from Folder failed', res.message || 'Unknown error');
      return;
    }

    const newColId = `col-${Date.now()}`;
    const newCol: Collection = {
      id: newColId,
      name: res.name || 'Imported Collection',
      folderPath,
      requests: res.requests.map(r => ({ ...r, collectionId: newColId }))
    };
    setCollections(prev => [...prev, newCol]);
    if (newCol.requests.length > 0) {
      setActiveRequest(newCol.requests[0]);
    }
    setActiveTab('api');
    handleRecordHistory(`Loaded from Folder: ${newCol.name}`, `${newCol.requests.length} requests from ${folderPath}`);
  };

  return (
    <div className="nexus-app-container">
      {/* Top Application Header */}
      <Header
        environments={environments}
        activeEnv={activeEnv}
        onSelectEnv={setActiveEnv}
        collections={collections}
        theme={theme}
        onToggleTheme={handleToggleTheme}
      />

      {/* Main App Cockpit */}
      <div className="nexus-body-layout" style={{ userSelect: isResizingSidebar ? 'none' : 'auto' }}>
        {/* Sidebar Nav with dynamic resizable width */}
        <div style={{ width: `${sidebarWidth}px`, height: '100%', flexShrink: 0 }}>
          <Sidebar
            activeTab={activeTab}
            onTabChange={setActiveTab}
            collections={collections}
            activeRequest={activeRequest}
            onSelectRequest={(req) => {
              setActiveRequest(req);
              setActiveTab('api');
            }}
            onNewRequest={handleNewRequest}
            onAddCollection={handleAddCollection}
            onDeleteCollection={handleDeleteCollection}
            onRenameCollection={handleRenameCollection}
            onNewRequestInCollection={handleNewRequestInCollection}
            onDeleteRequest={handleDeleteRequest}
            onRenameRequest={handleRenameRequest}
            onReorderCollections={handleReorderCollections}
            onReorderRequests={handleReorderRequests}
            onMoveRequest={handleMoveRequest}
            onImportCollection={handleImportCollection}
            onLinkCollectionFolder={handleLinkCollectionFolder}
            onUnlinkCollectionFolder={handleUnlinkCollectionFolder}
            onLoadCollectionFromFolder={handleLoadCollectionFromFolder}
            databases={databases}
            activeDb={activeDb}
            onSelectDb={(db) => {
              setActiveDb(db);
              setActiveTab('db');
            }}
            onAddDatabase={handleAddDatabase}
            onUpdateDatabase={handleUpdateDatabase}
            onDeleteDatabase={handleDeleteDatabase}
            onRenameDatabase={handleRenameDatabase}
            onReorderDatabases={handleReorderDatabases}
            environments={environments}
            activeEnv={activeEnv}
            onSelectEnv={setActiveEnv}
            onAddEnvironment={handleAddEnvironment}
            onDeleteEnvironment={handleDeleteEnvironment}
            onRenameEnvironment={handleRenameEnvironment}
            history={history}
            onClearHistory={handleClearHistory}
            onDeleteHistoryItem={handleDeleteHistoryItem}
            onReplayHistoryItem={handleReplayHistoryItem}
          />
        </div>

        {/* Vertical Resize Handle for Sidebar */}
        <div
          className={`pane-resizer-vertical ${isResizingSidebar ? 'resizing' : ''}`}
          onMouseDown={() => setIsResizingSidebar(true)}
          onDoubleClick={() => {
            setSidebarWidth(260);
            localStorage.setItem('bapu_sidebar_width', '260');
          }}
          title="Drag to resize sidebar • Double-click to reset (260px)"
        />

        {/* Main Central Workspace */}
        <main className="nexus-main-workspace">
          {/* Workspace Tabs Header */}
          <div className="workspace-tabs">
            <div 
              className={`workspace-tab ${activeTab === 'api' ? 'active' : ''}`}
              onClick={() => setActiveTab('api')}
            >
              <Globe size={13} color="#06b6d4" />
              <span>{activeRequest ? `${activeRequest.method} ${activeRequest.name}` : 'API Studio'}</span>
            </div>

            <div 
              className={`workspace-tab ${activeTab === 'db' ? 'active' : ''}`}
              onClick={() => setActiveTab('db')}
            >
              <Database size={13} color="#10b981" />
              <span>SQL Studio ({activeDb ? activeDb.name : 'No DB'})</span>
            </div>

            <div 
              className={`workspace-tab ${activeTab === 'streams' ? 'active' : ''}`}
              onClick={() => setActiveTab('streams')}
            >
              <Radio size={13} color="#a855f7" />
              <span>Stream Studio (SSE &amp; WS)</span>
            </div>

            <div 
              className={`workspace-tab ${activeTab === 'secrets' ? 'active' : ''}`}
              onClick={() => setActiveTab('secrets')}
            >
              <KeyRound size={13} color="#f59e0b" />
              <span>Secrets Matrix ({activeEnv.name})</span>
            </div>
          </div>

          {/* Active Canvas View */}
          <div style={{ flex: 1, overflow: 'hidden', display: 'flex', flexDirection: 'column' }}>
            {activeTab === 'api' && !activeRequest && (
              <div style={{
                flex: 1,
                display: 'flex',
                flexDirection: 'column',
                alignItems: 'center',
                justifyContent: 'center',
                gap: '14px',
                padding: '24px',
                textAlign: 'center'
              }}>
                <Globe size={40} color="#334155" />
                <div>
                  <div style={{ fontSize: '15px', fontWeight: 700, color: 'var(--text-main)', marginBottom: '6px' }}>
                    No request selected
                  </div>
                  <div style={{ fontSize: '12px', color: 'var(--text-dim)', maxWidth: '360px' }}>
                    Create a new request to start testing an API, or use the import icon in the sidebar
                    to bring in an existing Postman or OpenAPI collection.
                  </div>
                </div>
                <button onClick={handleNewRequest} className="btn-send" style={{ padding: '8px 18px' }}>
                  <Plus size={14} />
                  <span>New Request</span>
                </button>
              </div>
            )}

            {activeTab === 'api' && activeRequest && (
              <ErrorBoundary fallbackTitle="API Studio Encountered an Error">
                <ApiStudio
                  activeRequest={activeRequest}
                  activeEnv={activeEnv}
                  onUpdateRequest={handleUpdateRequest}
                  onRecordHistory={handleRecordHistory}
                  onDeleteRequest={handleDeleteRequest}
                  onUpdateEnv={(updatedEnv) => {
                    setEnvironments(prev => prev.map(e => e.id === updatedEnv.id ? updatedEnv : e));
                    setActiveEnv(updatedEnv);
                  }}
                />
              </ErrorBoundary>
            )}

            {activeTab === 'db' && (
              <ErrorBoundary fallbackTitle="Database Studio Encountered an Error">
                <DatabaseStudio
                  activeDb={activeDb}
                  onRecordHistory={handleRecordHistory}
                  onRenameDatabase={handleRenameDatabase}
                  onUpdateDatabase={handleUpdateDatabase}
                  onDatabaseLoaded={(newDb) => {
                    const cleanDb = sanitizeDatabase(newDb);
                    setDatabases(prev => [cleanDb, ...prev]);
                    setActiveDb(cleanDb);
                    setActiveTab('db');
                    handleRecordHistory(`Loaded SQLite: ${cleanDb.database}`, `${cleanDb.tables.length} tables`);
                  }}
                />
              </ErrorBoundary>
            )}

            {activeTab === 'streams' && (
              <ErrorBoundary fallbackTitle="Stream Studio Encountered an Error">
                <StreamStudio />
              </ErrorBoundary>
            )}

            {activeTab === 'secrets' && (
              <ErrorBoundary fallbackTitle="Secrets Studio Encountered an Error">
                <SecretsStudio
                  environments={environments}
                  activeEnv={activeEnv}
                  onUpdateEnvironment={(updated) => {
                    setEnvironments(prev => prev.map(e => e.id === updated.id ? updated : e));
                    setActiveEnv(updated);
                  }}
                />
              </ErrorBoundary>
            )}

            {activeTab === 'history' && (
              <ErrorBoundary fallbackTitle="History Studio Encountered an Error">
                <HistoryStudio
                  history={history}
                  onClearHistory={handleClearHistory}
                  onDeleteHistoryItem={handleDeleteHistoryItem}
                  onReplayItem={handleReplayHistoryItem}
                />
              </ErrorBoundary>
            )}
          </div>
        </main>
      </div>
    </div>
  );
};
export default App;
