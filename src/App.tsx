import React, { useState, useEffect } from 'react';
import { Header } from './components/Header';
import { Sidebar, WorkspaceTab } from './components/Sidebar';
import { ApiStudio } from './components/ApiStudio/ApiStudio';
import { DatabaseStudio } from './components/DatabaseStudio/DatabaseStudio';
import { StreamStudio } from './components/StreamStudio/StreamStudio';
import { SecretsStudio } from './components/SecretsStudio/SecretsStudio';
import { ProModal } from './components/ProModal';
import { 
  INITIAL_COLLECTIONS, 
  INITIAL_DATABASES, 
  INITIAL_ENVIRONMENTS, 
  INITIAL_HISTORY 
} from './data/mockData';
import { ApiRequest, Collection, DatabaseConnection, Environment, HistoryItem } from './types';
import { Globe, Database, KeyRound, Radio, Sparkles, X, Plus } from 'lucide-react';

export const App: React.FC = () => {
  // Navigation & Workspace State with LocalStorage Persistence
  const [activeTab, setActiveTab] = useState<WorkspaceTab>('api');

  const [collections, setCollections] = useState<Collection[]>(() => {
    try {
      const saved = localStorage.getItem('bapu_collections');
      if (saved) return JSON.parse(saved);
    } catch {}
    return INITIAL_COLLECTIONS;
  });

  const [activeRequest, setActiveRequest] = useState<ApiRequest | null>(() => {
    try {
      const saved = localStorage.getItem('bapu_collections');
      if (saved) {
        const parsed = JSON.parse(saved);
        if (parsed[0]?.requests[0]) return parsed[0].requests[0];
      }
    } catch {}
    return INITIAL_COLLECTIONS[0]?.requests[0] || null;
  });

  const [databases, setDatabases] = useState<DatabaseConnection[]>(() => {
    try {
      const saved = localStorage.getItem('bapu_databases');
      if (saved) return JSON.parse(saved);
    } catch {}
    return INITIAL_DATABASES;
  });

  const [activeDb, setActiveDb] = useState<DatabaseConnection>(() => {
    try {
      const saved = localStorage.getItem('bapu_databases');
      if (saved) {
        const parsed = JSON.parse(saved);
        if (parsed[0]) return parsed[0];
      }
    } catch {}
    return INITIAL_DATABASES[0];
  });

  const [environments, setEnvironments] = useState<Environment[]>(() => {
    try {
      const saved = localStorage.getItem('bapu_environments');
      if (saved) return JSON.parse(saved);
    } catch {}
    return INITIAL_ENVIRONMENTS;
  });

  const [activeEnv, setActiveEnv] = useState<Environment>(() => {
    try {
      const saved = localStorage.getItem('bapu_environments');
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

  const [isProModalOpen, setIsProModalOpen] = useState(false);

  // Automatically save any changes to localStorage
  useEffect(() => {
    try {
      localStorage.setItem('bapu_collections', JSON.stringify(collections));
    } catch {}
  }, [collections]);

  useEffect(() => {
    try {
      localStorage.setItem('bapu_databases', JSON.stringify(databases));
    } catch {}
  }, [databases]);

  useEffect(() => {
    try {
      localStorage.setItem('bapu_environments', JSON.stringify(environments));
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
      url: '{{API_BASE_URL}}/v1/resource',
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
    setDatabases(prev => [newDb, ...prev]);
    setActiveDb(newDb);
    setActiveTab('db');
    handleRecordHistory(`Connected: ${newDb.name}`, `${newDb.type.toUpperCase()} • ${newDb.database}`);
  };

  const handleDeleteDatabase = (dbId: string) => {
    setDatabases(prev => {
      const remaining = prev.filter(d => d.id !== dbId);
      if (activeDb.id === dbId && remaining.length > 0) {
        setActiveDb(remaining[0]);
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

  const handleImportCollection = (newCol: Collection) => {
    setCollections(prev => [...prev, newCol]);
    if (newCol.requests.length > 0) {
      setActiveRequest(newCol.requests[0]);
    }
    setActiveTab('api');
    handleRecordHistory(`Imported: ${newCol.name}`, `${newCol.requests.length} API endpoints parsed`);
  };

  return (
    <div className="nexus-app-container">
      {/* Top Application Header */}
      <Header
        environments={environments}
        activeEnv={activeEnv}
        onSelectEnv={setActiveEnv}
        onOpenProModal={() => setIsProModalOpen(true)}
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
            databases={databases}
            activeDb={activeDb}
            onSelectDb={(db) => {
              setActiveDb(db);
              setActiveTab('db');
            }}
            onAddDatabase={handleAddDatabase}
            onDeleteDatabase={handleDeleteDatabase}
            onRenameDatabase={handleRenameDatabase}
            onReorderDatabases={handleReorderDatabases}
            history={history}
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
            {activeTab === 'api' && activeRequest && (
              <ApiStudio
                activeRequest={activeRequest}
                activeEnv={activeEnv}
                onUpdateRequest={handleUpdateRequest}
                onRecordHistory={handleRecordHistory}
                onDeleteRequest={handleDeleteRequest}
              />
            )}

            {activeTab === 'db' && (
              <DatabaseStudio
                activeDb={activeDb}
                onRecordHistory={handleRecordHistory}
                onRenameDatabase={handleRenameDatabase}
                onDatabaseLoaded={(newDb) => {
                  setDatabases(prev => [newDb, ...prev]);
                  setActiveDb(newDb);
                  setActiveTab('db');
                  handleRecordHistory(`Loaded SQLite: ${newDb.database}`, `${newDb.tables.length} tables`);
                }}
              />
            )}

            {activeTab === 'streams' && (
              <StreamStudio />
            )}

            {activeTab === 'secrets' && (
              <SecretsStudio
                environments={environments}
                activeEnv={activeEnv}
                onUpdateEnvironment={(updated) => {
                  setEnvironments(prev => prev.map(e => e.id === updated.id ? updated : e));
                  setActiveEnv(updated);
                }}
              />
            )}

            {activeTab === 'history' && (
              <div style={{ padding: '24px', maxWidth: '800px', margin: '0 auto', width: '100%' }}>
                <h2 style={{ fontSize: '18px', fontWeight: 700, color: '#fff', marginBottom: '16px' }}>
                  Execution History
                </h2>
                <div style={{ display: 'flex', flexDirection: 'column', gap: '8px' }}>
                  {history.map(item => (
                    <div key={item.id} style={{
                      display: 'flex',
                      alignItems: 'center',
                      justifyContent: 'space-between',
                      padding: '12px 16px',
                      background: 'var(--bg-card)',
                      border: '1px solid var(--border-subtle)',
                      borderRadius: 'var(--radius-md)'
                    }}>
                      <div>
                        <div style={{ fontWeight: 600, color: '#fff' }}>{item.title}</div>
                        <div style={{ fontSize: '11px', color: 'var(--text-dim)', marginTop: '2px' }}>{item.subtitle}</div>
                      </div>
                      <span style={{ fontSize: '11px', color: 'var(--text-muted)' }}>{item.timestamp}</span>
                    </div>
                  ))}
                </div>
              </div>
            )}
          </div>
        </main>
      </div>

      {/* Commercial Open-Core Upgrade Modal */}
      <ProModal
        isOpen={isProModalOpen}
        onClose={() => setIsProModalOpen(false)}
      />
    </div>
  );
};
export default App;
