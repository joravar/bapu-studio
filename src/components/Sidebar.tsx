import React, { useState } from 'react';
import { 
  Globe, 
  Database, 
  KeyRound, 
  History, 
  FolderPlus, 
  Plus, 
  ChevronRight, 
  ChevronDown, 
  Folder, 
  Trash2, 
  HardDrive, 
  Radio, 
  GripVertical,
  Upload,
  Download,
  FolderDown,
  Edit3,
  Settings,
  Play,
  Copy,
  Check
} from 'lucide-react';
import { Collection, ApiRequest, DatabaseConnection, HistoryItem, Environment, KeyValuePair } from '../types';
import { NewCollectionModal } from './ApiStudio/NewCollectionModal';
import { NewConnectionModal } from './DatabaseStudio/NewConnectionModal';
import { ImportExportModal } from './ApiStudio/ImportExportModal';

export type WorkspaceTab = 'api' | 'db' | 'streams' | 'secrets' | 'history';

interface SidebarProps {
  activeTab: WorkspaceTab;
  onTabChange: (tab: WorkspaceTab) => void;
  collections: Collection[];
  activeRequest: ApiRequest | null;
  onSelectRequest: (req: ApiRequest) => void;
  onNewRequest: () => void;
  onAddCollection: (name: string) => void;
  onDeleteCollection: (collectionId: string) => void;
  onRenameCollection?: (collectionId: string, newName: string) => void;
  onNewRequestInCollection: (collectionId: string) => void;
  onDeleteRequest: (requestId: string) => void;
  onRenameRequest?: (requestId: string, newName: string) => void;
  onReorderCollections?: (sourceIndex: number, destIndex: number) => void;
  onReorderRequests?: (collectionId: string, sourceIndex: number, destIndex: number) => void;
  onMoveRequest?: (sourceColId: string, destColId: string, sourceIndex: number, destIndex: number) => void;
  onImportCollection?: (collection: Collection) => void;
  databases: DatabaseConnection[];
  activeDb: DatabaseConnection | null;
  onSelectDb: (db: DatabaseConnection) => void;
  onAddDatabase: (db: DatabaseConnection) => void;
  onUpdateDatabase?: (db: DatabaseConnection) => void;
  onDeleteDatabase: (dbId: string) => void;
  onRenameDatabase?: (dbId: string, newName: string) => void;
  onReorderDatabases?: (sourceIndex: number, destIndex: number) => void;
  environments?: Environment[];
  activeEnv?: Environment;
  onSelectEnv?: (env: Environment) => void;
  onAddEnvironment?: (name: string) => void;
  onDeleteEnvironment?: (envId: string) => void;
  onRenameEnvironment?: (envId: string, newName: string) => void;
  history: HistoryItem[];
  onClearHistory?: () => void;
  onDeleteHistoryItem?: (id: string) => void;
  onReplayHistoryItem?: (item: HistoryItem) => void;
}

export const Sidebar: React.FC<SidebarProps> = ({
  activeTab,
  onTabChange,
  collections,
  activeRequest,
  onSelectRequest,
  onNewRequest,
  onAddCollection,
  onDeleteCollection,
  onRenameCollection,
  onNewRequestInCollection,
  onDeleteRequest,
  onRenameRequest,
  onReorderCollections,
  onReorderRequests,
  onMoveRequest,
  onImportCollection,
  databases,
  activeDb,
  onSelectDb,
  onAddDatabase,
  onUpdateDatabase,
  onDeleteDatabase,
  onRenameDatabase,
  onReorderDatabases,
  environments = [],
  activeEnv,
  onSelectEnv,
  onAddEnvironment,
  onDeleteEnvironment,
  onRenameEnvironment,
  history,
  onClearHistory,
  onDeleteHistoryItem,
  onReplayHistoryItem
}) => {
  const [isNewColModalOpen, setIsNewColModalOpen] = useState(false);
  const [isNewDbModalOpen, setIsNewDbModalOpen] = useState(false);
  const [editingDbConfig, setEditingDbConfig] = useState<DatabaseConnection | null>(null);
  const [isNewEnvModalOpen, setIsNewEnvModalOpen] = useState(false);
  const [newEnvName, setNewEnvName] = useState('');
  const [isImportExportModalOpen, setIsImportExportModalOpen] = useState(false);
  const [selectedColForExport, setSelectedColForExport] = useState<string | undefined>(undefined);
  const [collapsedCols, setCollapsedCols] = useState<Record<string, boolean>>({});

  // Inline Rename States
  const [editingColId, setEditingColId] = useState<string | null>(null);
  const [editingColName, setEditingColName] = useState('');
  const [editingReqId, setEditingReqId] = useState<string | null>(null);
  const [editingReqName, setEditingReqName] = useState('');
  const [editingDbId, setEditingDbId] = useState<string | null>(null);
  const [editingDbName, setEditingDbName] = useState('');
  const [editingEnvId, setEditingEnvId] = useState<string | null>(null);
  const [editingEnvName, setEditingEnvName] = useState('');

  // Drag and Drop States
  const [draggedColIndex, setDraggedColIndex] = useState<number | null>(null);
  const [dragOverColIndex, setDragOverColIndex] = useState<number | null>(null);

  const [draggedReq, setDraggedReq] = useState<{ colId: string; index: number } | null>(null);
  const [dragOverReq, setDragOverReq] = useState<{ colId: string; index: number } | null>(null);

  const [draggedDbIndex, setDraggedDbIndex] = useState<number | null>(null);
  const [dragOverDbIndex, setDragOverDbIndex] = useState<number | null>(null);

  const toggleCollapse = (id: string) => {
    setCollapsedCols(prev => ({ ...prev, [id]: !prev[id] }));
  };

  return (
    <aside className="nexus-sidebar">
      {/* Top Engine Navigation Tabs */}
      <div className="sidebar-nav-tabs">
        <button
          className={`sidebar-nav-tab ${activeTab === 'api' ? 'active' : ''}`}
          onClick={() => onTabChange('api')}
          title="API Client Studio"
        >
          <Globe size={15} />
          <span>APIs</span>
        </button>

        <button
          className={`sidebar-nav-tab ${activeTab === 'db' ? 'active' : ''}`}
          onClick={() => onTabChange('db')}
          title="SQL & Database Studio"
        >
          <Database size={15} />
          <span>Databases</span>
        </button>

        <button
          className={`sidebar-nav-tab ${activeTab === 'streams' ? 'active' : ''}`}
          onClick={() => onTabChange('streams')}
          title="Real-Time WebSocket & AI Stream Studio"
        >
          <Radio size={15} />
          <span>Streams</span>
        </button>

        <button
          className={`sidebar-nav-tab ${activeTab === 'secrets' ? 'active' : ''}`}
          onClick={() => onTabChange('secrets')}
          title="Secrets & Environment Vault"
        >
          <KeyRound size={15} />
          <span>Secrets</span>
        </button>

        <button
          className={`sidebar-nav-tab ${activeTab === 'history' ? 'active' : ''}`}
          onClick={() => onTabChange('history')}
          title="Request & Query History"
        >
          <History size={15} />
          <span>History</span>
        </button>
      </div>

      {/* Dynamic Sidebar Content */}
      <div className="sidebar-content">
        {activeTab === 'api' && (
          <div>
            <div className="sidebar-section-header">
              <span>Collections ({collections.length})</span>
              <div style={{ display: 'flex', gap: '4px' }}>
                <button 
                  onClick={() => {
                    setSelectedColForExport(undefined);
                    setIsImportExportModalOpen(true);
                  }} 
                  className="sidebar-action-btn" 
                  title="Import / Export Collections (Postman, OpenAPI, Bapu JSON)"
                >
                  <FolderDown size={14} color="#38bdf8" />
                </button>
                <button 
                  onClick={onNewRequest} 
                  className="sidebar-action-btn" 
                  title="New HTTP Request"
                >
                  <Plus size={14} />
                </button>
                <button 
                  onClick={() => setIsNewColModalOpen(true)}
                  className="sidebar-action-btn" 
                  title="Create New Collection Folder"
                >
                  <FolderPlus size={14} color="#60a5fa" />
                </button>
              </div>
            </div>

            {collections.map((col, colIndex) => {
              const isCollapsed = !!collapsedCols[col.id];
              const isDraggingThisCol = draggedColIndex === colIndex;
              const isDragOverCol = dragOverColIndex === colIndex;

              return (
                <div
                  key={col.id}
                  style={{
                    marginBottom: '10px',
                    opacity: isDraggingThisCol ? 0.4 : 1,
                    borderTop: isDragOverCol && draggedColIndex !== null && draggedColIndex !== colIndex ? '2px solid #3b82f6' : 'none',
                    transition: 'all 0.15s ease'
                  }}
                  onDragOver={(e) => {
                    e.preventDefault();
                    if (draggedColIndex !== null && draggedColIndex !== colIndex) {
                      setDragOverColIndex(colIndex);
                    }
                  }}
                  onDragLeave={() => {
                    if (dragOverColIndex === colIndex) setDragOverColIndex(null);
                  }}
                  onDrop={(e) => {
                    e.preventDefault();
                    if (draggedColIndex !== null && draggedColIndex !== colIndex) {
                      if (onReorderCollections) onReorderCollections(draggedColIndex, colIndex);
                      setDraggedColIndex(null);
                      setDragOverColIndex(null);
                    }
                  }}
                >
                  <div
                    draggable={!editingColId}
                    onDragStart={(e) => {
                      e.dataTransfer.effectAllowed = 'move';
                      setDraggedColIndex(colIndex);
                    }}
                    onDragEnd={() => {
                      setDraggedColIndex(null);
                      setDragOverColIndex(null);
                    }}
                    style={{
                      display: 'flex',
                      alignItems: 'center',
                      justifyContent: 'space-between',
                      padding: '4px 6px',
                      borderRadius: 'var(--radius-sm)',
                      fontSize: '12px',
                      fontWeight: 600,
                      color: 'var(--text-muted)',
                      cursor: 'grab'
                    }}
                  >
                    <div 
                      style={{ display: 'flex', alignItems: 'center', gap: '6px', cursor: 'pointer', flex: 1, overflow: 'hidden' }}
                    >
                      <GripVertical size={11} style={{ opacity: 0.4, cursor: 'grab', marginRight: '-2px' }} />
                      <div 
                        onClick={() => toggleCollapse(col.id)}
                        style={{ display: 'flex', alignItems: 'center', cursor: 'pointer' }}
                      >
                        {isCollapsed ? <ChevronRight size={13} /> : <ChevronDown size={13} />}
                        <Folder size={14} color="#60a5fa" style={{ marginLeft: '4px' }} />
                      </div>
                      
                      {editingColId === col.id ? (
                        <input
                          autoFocus
                          value={editingColName}
                          onChange={(e) => setEditingColName(e.target.value)}
                          onFocus={(e) => e.target.select()}
                          onClick={(e) => e.stopPropagation()}
                          onMouseDown={(e) => e.stopPropagation()}
                          onKeyDown={(e) => {
                            if (e.key === 'Enter') {
                              if (onRenameCollection && editingColName.trim()) {
                                onRenameCollection(col.id, editingColName.trim());
                              }
                              setEditingColId(null);
                            } else if (e.key === 'Escape') {
                              setEditingColId(null);
                            }
                          }}
                          onBlur={() => {
                            if (onRenameCollection && editingColName.trim()) {
                              onRenameCollection(col.id, editingColName.trim());
                            }
                            setEditingColId(null);
                          }}
                          className="inline-rename-input"
                          style={{ height: '22px', fontSize: '11px', flex: 1 }}
                        />
                      ) : (
                        <span 
                          onClick={() => toggleCollapse(col.id)}
                          onMouseDown={(e) => e.stopPropagation()}
                          onDoubleClick={(e) => {
                            e.preventDefault();
                            e.stopPropagation();
                            setEditingColId(col.id);
                            setEditingColName(col.name);
                          }}
                          title="Double-click to rename collection"
                          style={{ overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap', flex: 1 }}
                        >
                          {col.name}
                        </span>
                      )}
                    </div>

                    <div style={{ display: 'flex', alignItems: 'center', gap: '2px' }}>
                      <button
                        onClick={(e) => {
                          e.stopPropagation();
                          setEditingColId(col.id);
                          setEditingColName(col.name);
                        }}
                        className="sidebar-action-btn"
                        title={`Rename "${col.name}"`}
                      >
                        <Edit3 size={11} color="var(--text-dim)" />
                      </button>
                      <button
                        onClick={(e) => {
                          e.stopPropagation();
                          setSelectedColForExport(col.id);
                          setIsImportExportModalOpen(true);
                        }}
                        className="sidebar-action-btn"
                        title={`Export "${col.name}" (Postman / OpenAPI / JSON)`}
                      >
                        <Download size={11} color="#10b981" />
                      </button>
                      <button
                        onClick={() => onNewRequestInCollection(col.id)}
                        className="sidebar-action-btn"
                        title={`Add Request to ${col.name}`}
                      >
                        <Plus size={12} />
                      </button>
                      {(
                        <button
                          onClick={() => {
                            if (confirm(`Delete collection "${col.name}"?`)) {
                              onDeleteCollection(col.id);
                            }
                          }}
                          className="sidebar-action-btn"
                          title="Delete Collection"
                        >
                          <Trash2 size={11} />
                        </button>
                      )}
                    </div>
                  </div>

                  {!isCollapsed && (
                    <div
                      style={{ paddingLeft: '14px', marginTop: '2px', minHeight: col.requests.length === 0 ? '30px' : 'auto' }}
                      onDragOver={(e) => {
                        e.preventDefault();
                        if (draggedReq && draggedReq.colId !== col.id && col.requests.length === 0) {
                          setDragOverReq({ colId: col.id, index: 0 });
                        }
                      }}
                      onDrop={(e) => {
                        e.preventDefault();
                        if (draggedReq && draggedReq.colId !== col.id && col.requests.length === 0) {
                          if (onMoveRequest) onMoveRequest(draggedReq.colId, col.id, draggedReq.index, 0);
                          setDraggedReq(null);
                          setDragOverReq(null);
                        }
                      }}
                    >
                      {col.requests.length === 0 ? (
                        <div style={{ fontSize: '11px', color: 'var(--text-dim)', padding: '4px 8px', border: '1px dashed var(--border-subtle)', borderRadius: '4px', margin: '4px 0' }}>
                          Drag requests here or click + above
                        </div>
                      ) : (
                        col.requests.map((req, reqIndex) => {
                          const isSelected = activeRequest?.id === req.id;
                          const isDraggingThisReq = draggedReq?.colId === col.id && draggedReq.index === reqIndex;
                          const isOverThisReq = dragOverReq?.colId === col.id && dragOverReq.index === reqIndex;

                          return (
                            <div
                              key={req.id}
                              draggable={!editingReqId}
                              onDragStart={(e) => {
                                e.stopPropagation();
                                e.dataTransfer.effectAllowed = 'move';
                                setDraggedReq({ colId: col.id, index: reqIndex });
                              }}
                              onDragEnd={() => {
                                setDraggedReq(null);
                                setDragOverReq(null);
                              }}
                              onDragOver={(e) => {
                                e.preventDefault();
                                e.stopPropagation();
                                if (draggedReq && (draggedReq.colId !== col.id || draggedReq.index !== reqIndex)) {
                                  setDragOverReq({ colId: col.id, index: reqIndex });
                                }
                              }}
                              onDragLeave={(e) => {
                                e.stopPropagation();
                                if (dragOverReq?.colId === col.id && dragOverReq.index === reqIndex) {
                                  setDragOverReq(null);
                                }
                              }}
                              onDrop={(e) => {
                                e.preventDefault();
                                e.stopPropagation();
                                if (draggedReq) {
                                  if (draggedReq.colId === col.id) {
                                    if (onReorderRequests) onReorderRequests(col.id, draggedReq.index, reqIndex);
                                  } else {
                                    if (onMoveRequest) onMoveRequest(draggedReq.colId, col.id, draggedReq.index, reqIndex);
                                  }
                                  setDraggedReq(null);
                                  setDragOverReq(null);
                                }
                              }}
                              className={`sidebar-item ${isSelected ? 'active' : ''}`}
                              onClick={() => onSelectRequest(req)}
                              style={{
                                display: 'flex',
                                alignItems: 'center',
                                justifyContent: 'space-between',
                                opacity: isDraggingThisReq ? 0.35 : 1,
                                borderTop: isOverThisReq ? '2px solid #38bdf8' : 'none',
                                cursor: 'grab',
                                transition: 'all 0.15s ease'
                              }}
                            >
                              <div className="sidebar-item-left" style={{ flex: 1, minWidth: 0, display: 'flex', alignItems: 'center', gap: '6px' }}>
                                <GripVertical size={11} style={{ opacity: 0.35, flexShrink: 0 }} />
                                <span className={`method-pill method-${req.method}`}>{req.method}</span>
                                {editingReqId === req.id ? (
                                  <input
                                    autoFocus
                                    value={editingReqName}
                                    onChange={(e) => setEditingReqName(e.target.value)}
                                    onFocus={(e) => e.target.select()}
                                    onClick={(e) => e.stopPropagation()}
                                    onMouseDown={(e) => e.stopPropagation()}
                                    onKeyDown={(e) => {
                                      if (e.key === 'Enter') {
                                        if (onRenameRequest && editingReqName.trim()) {
                                          onRenameRequest(req.id, editingReqName.trim());
                                        }
                                        setEditingReqId(null);
                                      } else if (e.key === 'Escape') {
                                        setEditingReqId(null);
                                      }
                                    }}
                                    onBlur={() => {
                                      if (onRenameRequest && editingReqName.trim()) {
                                        onRenameRequest(req.id, editingReqName.trim());
                                      }
                                      setEditingReqId(null);
                                    }}
                                    className="inline-rename-input"
                                    style={{ height: '20px', fontSize: '11px', flex: 1 }}
                                  />
                                ) : (
                                  <span 
                                    onMouseDown={(e) => e.stopPropagation()}
                                    onDoubleClick={(e) => {
                                      e.preventDefault();
                                      e.stopPropagation();
                                      setEditingReqId(req.id);
                                      setEditingReqName(req.name);
                                    }}
                                    title="Double-click to rename request"
                                    style={{ fontSize: '12px', overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap', flex: 1 }}
                                  >
                                    {req.name}
                                  </span>
                                )}
                              </div>

                              <div style={{ display: 'flex', alignItems: 'center', gap: '2px' }}>
                                <button
                                  onClick={(e) => {
                                    e.stopPropagation();
                                    setEditingReqId(req.id);
                                    setEditingReqName(req.name);
                                  }}
                                  className="sidebar-action-btn"
                                  title={`Rename "${req.name}"`}
                                  style={{ opacity: isSelected ? 1 : 0.6 }}
                                >
                                  <Edit3 size={11} color="var(--text-dim)" />
                                </button>
                                <button
                                  onClick={(e) => {
                                    e.stopPropagation();
                                    if (confirm(`Delete request "${req.name}"?`)) {
                                      onDeleteRequest(req.id);
                                    }
                                  }}
                                  className="sidebar-action-btn"
                                  title="Delete Request"
                                  style={{ opacity: isSelected ? 1 : 0.6 }}
                                >
                                  <Trash2 size={11} />
                                </button>
                              </div>
                            </div>
                          );
                        })
                      )}
                    </div>
                  )}
                </div>
              );
            })}
          </div>
        )}

        {activeTab === 'db' && (
          <div>
            <div className="sidebar-section-header">
              <span>Connected Drivers ({databases.length})</span>
              <button 
                onClick={() => setIsNewDbModalOpen(true)}
                className="sidebar-action-btn" 
                title="Add Database Connection"
              >
                <Plus size={14} color="#10b981" />
              </button>
            </div>

            {databases.map((db, dbIndex) => {
              const isSelected = activeDb?.id === db.id;
              const isDraggingThisDb = draggedDbIndex === dbIndex;
              const isOverThisDb = dragOverDbIndex === dbIndex;

              return (
                <div
                  key={db.id}
                  draggable={!editingDbId}
                  onDragStart={(e) => {
                    e.dataTransfer.effectAllowed = 'move';
                    setDraggedDbIndex(dbIndex);
                  }}
                  onDragEnd={() => {
                    setDraggedDbIndex(null);
                    setDragOverDbIndex(null);
                  }}
                  onDragOver={(e) => {
                    e.preventDefault();
                    if (draggedDbIndex !== null && draggedDbIndex !== dbIndex) {
                      setDragOverDbIndex(dbIndex);
                    }
                  }}
                  onDragLeave={() => {
                    if (dragOverDbIndex === dbIndex) setDragOverDbIndex(null);
                  }}
                  onDrop={(e) => {
                    e.preventDefault();
                    if (draggedDbIndex !== null && draggedDbIndex !== dbIndex) {
                      if (onReorderDatabases) onReorderDatabases(draggedDbIndex, dbIndex);
                      setDraggedDbIndex(null);
                      setDragOverDbIndex(null);
                    }
                  }}
                  className={`sidebar-item ${isSelected ? 'active' : ''}`}
                  onClick={() => onSelectDb(db)}
                  style={{
                    padding: '8px',
                    marginBottom: '6px',
                    display: 'flex',
                    alignItems: 'center',
                    justifyContent: 'space-between',
                    opacity: isDraggingThisDb ? 0.35 : 1,
                    borderTop: isOverThisDb ? '2px solid #10b981' : 'none',
                    cursor: 'grab',
                    transition: 'all 0.15s ease'
                  }}
                >
                  <div 
                    className="sidebar-item-left" 
                    onMouseDown={(e) => e.stopPropagation()}
                    onDoubleClick={(e) => {
                      e.preventDefault();
                      e.stopPropagation();
                      setEditingDbId(db.id);
                      setEditingDbName(db.name);
                    }}
                    style={{ display: 'flex', alignItems: 'center', gap: '6px', flex: 1, minWidth: 0, cursor: 'pointer' }}
                  >
                    <GripVertical size={11} style={{ opacity: 0.35, flexShrink: 0 }} />
                    <HardDrive size={14} color={db.isConnected ? '#10b981' : '#64748b'} style={{ flexShrink: 0 }} />
                    <div style={{ flex: 1, minWidth: 0 }}>
                      {editingDbId === db.id ? (
                        <input
                          autoFocus
                          value={editingDbName}
                          onChange={(e) => setEditingDbName(e.target.value)}
                          onFocus={(e) => e.target.select()}
                          onClick={(e) => e.stopPropagation()}
                          onMouseDown={(e) => e.stopPropagation()}
                          onKeyDown={(e) => {
                            if (e.key === 'Enter') {
                              if (onRenameDatabase && editingDbName.trim()) {
                                onRenameDatabase(db.id, editingDbName.trim());
                              }
                              setEditingDbId(null);
                            } else if (e.key === 'Escape') {
                              setEditingDbId(null);
                            }
                          }}
                          onBlur={() => {
                            if (onRenameDatabase && editingDbName.trim()) {
                              onRenameDatabase(db.id, editingDbName.trim());
                            }
                            setEditingDbId(null);
                          }}
                          className="inline-rename-input"
                          style={{ height: '20px', fontSize: '11px', width: '100%' }}
                        />
                      ) : (
                        <div 
                          title="Double-click to rename connection"
                          style={{ fontSize: '12px', fontWeight: 600, overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}
                        >
                          {db.name}
                        </div>
                      )}
                      <div style={{ fontSize: '10px', color: 'var(--text-dim)', fontFamily: 'var(--font-mono)' }}>
                        {db.type.toUpperCase()} • {db.database}
                      </div>
                    </div>
                  </div>

                  <div style={{ display: 'flex', alignItems: 'center', gap: '2px' }}>
                    <button
                      onClick={(e) => {
                        e.stopPropagation();
                        setEditingDbConfig(db);
                        setIsNewDbModalOpen(true);
                      }}
                      className="sidebar-action-btn"
                      title={`Edit connection parameters for "${db.name}"`}
                      style={{ opacity: isSelected ? 1 : 0.6 }}
                    >
                      <Settings size={11} color="var(--text-dim)" />
                    </button>
                    <button
                      onClick={(e) => {
                        e.stopPropagation();
                        setEditingDbId(db.id);
                        setEditingDbName(db.name);
                      }}
                      className="sidebar-action-btn"
                      title={`Rename "${db.name}"`}
                      style={{ opacity: isSelected ? 1 : 0.6 }}
                    >
                      <Edit3 size={11} color="var(--text-dim)" />
                    </button>
                    <button
                      onClick={(e) => {
                        e.stopPropagation();
                        if (confirm(`Remove database connection "${db.name}"?`)) {
                          onDeleteDatabase(db.id);
                        }
                      }}
                      className="sidebar-action-btn"
                      title="Remove Connection"
                    >
                      <Trash2 size={11} />
                    </button>
                  </div>
                </div>
              );
            })}
          </div>
        )}

        {activeTab === 'secrets' && (
          <div>
            <div className="sidebar-section-header">
              <span>Environments ({environments.length})</span>
              <button 
                onClick={() => setIsNewEnvModalOpen(true)}
                className="sidebar-action-btn" 
                title="Create New Environment"
              >
                <Plus size={14} color="#f59e0b" />
              </button>
            </div>

            {environments.map((env) => {
              const isSelected = activeEnv?.id === env.id;

              return (
                <div key={env.id} style={{ marginBottom: '8px' }}>
                  <div
                    className={`sidebar-item ${isSelected ? 'active' : ''}`}
                    onClick={() => onSelectEnv && onSelectEnv(env)}
                    style={{
                      padding: '7px 8px',
                      display: 'flex',
                      alignItems: 'center',
                      justifyContent: 'space-between',
                      cursor: 'pointer'
                    }}
                  >
                    <div 
                      className="sidebar-item-left" 
                      onMouseDown={(e) => e.stopPropagation()}
                      onDoubleClick={(e) => {
                        e.preventDefault();
                        e.stopPropagation();
                        setEditingEnvId(env.id);
                        setEditingEnvName(env.name);
                      }}
                      style={{ display: 'flex', alignItems: 'center', gap: '6px', flex: 1, minWidth: 0 }}
                    >
                      <KeyRound size={13} color={isSelected ? '#f59e0b' : '#64748b'} style={{ flexShrink: 0 }} />
                      <div style={{ flex: 1, minWidth: 0 }}>
                        {editingEnvId === env.id ? (
                          <input
                            autoFocus
                            value={editingEnvName}
                            onChange={(e) => setEditingEnvName(e.target.value)}
                            onFocus={(e) => e.target.select()}
                            onClick={(e) => e.stopPropagation()}
                            onMouseDown={(e) => e.stopPropagation()}
                            onKeyDown={(e) => {
                              if (e.key === 'Enter') {
                                if (onRenameEnvironment && editingEnvName.trim()) {
                                  onRenameEnvironment(env.id, editingEnvName.trim());
                                }
                                setEditingEnvId(null);
                              } else if (e.key === 'Escape') {
                                setEditingEnvId(null);
                              }
                            }}
                            onBlur={() => {
                              if (onRenameEnvironment && editingEnvName.trim()) {
                                onRenameEnvironment(env.id, editingEnvName.trim());
                              }
                              setEditingEnvId(null);
                            }}
                            className="inline-rename-input"
                            style={{ height: '20px', fontSize: '11px', width: '100%' }}
                          />
                        ) : (
                          <div 
                            title="Double-click to rename environment"
                            style={{ fontSize: '12px', fontWeight: 600, overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}
                          >
                            {env.name}
                          </div>
                        )}
                        <div style={{ fontSize: '10px', color: 'var(--text-dim)', fontFamily: 'var(--font-mono)' }}>
                          {env.variables.length} variable{env.variables.length === 1 ? '' : 's'}
                        </div>
                      </div>
                    </div>

                    <div style={{ display: 'flex', alignItems: 'center', gap: '2px' }}>
                      <button
                        onClick={(e) => {
                          e.stopPropagation();
                          setEditingEnvId(env.id);
                          setEditingEnvName(env.name);
                        }}
                        className="sidebar-action-btn"
                        title={`Rename "${env.name}"`}
                        style={{ opacity: isSelected ? 1 : 0.6 }}
                      >
                        <Edit3 size={11} color="var(--text-dim)" />
                      </button>
                      {environments.length > 1 && (
                        <button
                          onClick={(e) => {
                            e.stopPropagation();
                            if (confirm(`Delete environment "${env.name}"?`)) {
                              if (onDeleteEnvironment) onDeleteEnvironment(env.id);
                            }
                          }}
                          className="sidebar-action-btn"
                          title="Delete Environment"
                        >
                          <Trash2 size={11} />
                        </button>
                      )}
                    </div>
                  </div>

                  {/* If active, display live variables list directly in sidebar */}
                  {isSelected && (
                    <div style={{ paddingLeft: '14px', marginTop: '4px' }}>
                      {env.variables.length === 0 ? (
                        <div style={{ fontSize: '11px', color: 'var(--text-dim)', fontStyle: 'italic', padding: '4px 6px' }}>
                          No variables yet. Use bapu.env.set(...) in scripts or Add Variable in Secrets Studio.
                        </div>
                      ) : (
                        env.variables.map((v: KeyValuePair) => (
                          <div
                            key={v.id}
                            style={{
                              display: 'flex',
                              alignItems: 'center',
                              justifyContent: 'space-between',
                              padding: '3px 6px',
                              borderRadius: 'var(--radius-sm)',
                              fontSize: '11px',
                              background: 'rgba(255, 255, 255, 0.02)',
                              marginBottom: '3px'
                            }}
                          >
                            <div style={{ display: 'flex', alignItems: 'center', gap: '6px', overflow: 'hidden', flex: 1 }}>
                              <span style={{ 
                                width: '6px', 
                                height: '6px', 
                                borderRadius: '50%', 
                                background: v.enabled ? '#10b981' : '#64748b', 
                                flexShrink: 0 
                              }} />
                              <span style={{ 
                                fontFamily: 'var(--font-mono)', 
                                color: v.isSecret ? '#f59e0b' : '#38bdf8', 
                                fontWeight: 500, 
                                overflow: 'hidden', 
                                textOverflow: 'ellipsis', 
                                whiteSpace: 'nowrap' 
                              }}>
                                {v.key}
                              </span>
                            </div>
                            <span style={{ 
                              fontSize: '9px', 
                              color: 'var(--text-dim)', 
                              fontFamily: 'var(--font-mono)', 
                              marginLeft: '6px', 
                              flexShrink: 0 
                            }}>
                              {v.isSecret ? '••••' : (v.value ? (v.value.length > 8 ? v.value.slice(0, 8) + '...' : v.value) : '(empty)')}
                            </span>
                          </div>
                        ))
                      )}
                    </div>
                  )}
                </div>
              );
            })}
          </div>
        )}

        {activeTab === 'history' && (
          <div>
            <div className="sidebar-section-header">
              <span>Recent Activity ({history.length})</span>
              {history.length > 0 && (
                <button
                  onClick={() => {
                    if (confirm('Clear all activity history?')) {
                      if (onClearHistory) onClearHistory();
                    }
                  }}
                  className="sidebar-action-btn"
                  title="Clear All History"
                >
                  <Trash2 size={13} />
                </button>
              )}
            </div>

            {history.length === 0 ? (
              <div style={{ padding: '24px 12px', textAlign: 'center', color: 'var(--text-dim)', fontSize: '11px' }}>
                No recent activity recorded yet.
              </div>
            ) : (
              history.map(item => (
                <div
                  key={item.id}
                  className="sidebar-item"
                  onClick={() => onReplayHistoryItem && onReplayHistoryItem(item)}
                  style={{
                    padding: '6px 8px',
                    display: 'flex',
                    alignItems: 'center',
                    justifyContent: 'space-between',
                    cursor: 'pointer'
                  }}
                >
                  <div style={{ flex: 1, minWidth: 0, marginRight: '6px' }}>
                    <div style={{ fontSize: '12px', fontWeight: 500, color: 'var(--text-main)', overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>
                      {item.title}
                    </div>
                    <div style={{ fontSize: '10px', color: 'var(--text-dim)', marginTop: '2px' }}>
                      {item.subtitle} • {item.timestamp}
                    </div>
                  </div>

                  <div style={{ display: 'flex', alignItems: 'center', gap: '2px', flexShrink: 0 }}>
                    <button
                      onClick={(e) => {
                        e.stopPropagation();
                        if (onReplayHistoryItem) onReplayHistoryItem(item);
                      }}
                      className="sidebar-action-btn"
                      title="Replay in Studio"
                    >
                      <Play size={10} color="#10b981" />
                    </button>
                    {onDeleteHistoryItem && (
                      <button
                        onClick={(e) => {
                          e.stopPropagation();
                          onDeleteHistoryItem(item.id);
                        }}
                        className="sidebar-action-btn"
                        title="Delete from history"
                      >
                        <Trash2 size={10} />
                      </button>
                    )}
                  </div>
                </div>
              ))
            )}
          </div>
        )}
      </div>

      {/* Footer Info */}
      <div style={{
        padding: '10px 12px',
        borderTop: '1px solid var(--border-subtle)',
        fontSize: '11px',
        color: 'var(--text-dim)',
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'space-between'
      }}>
        <span>RAM: <strong>28.4 MB</strong></span>
        <span>v1.0.0-oss</span>
      </div>

      {/* New Environment Modal */}
      {isNewEnvModalOpen && (
        <div className="modal-overlay">
          <div className="modal-content" style={{ maxWidth: '380px' }}>
            <h3 style={{ fontSize: '15px', fontWeight: 700, marginBottom: '12px' }}>Create New Environment</h3>
            <p style={{ fontSize: '12px', color: 'var(--text-muted)', marginBottom: '12px' }}>
              Create an isolated scope for staging, production, or local secrets.
            </p>
            <input
              type="text"
              autoFocus
              value={newEnvName}
              onChange={(e) => setNewEnvName(e.target.value)}
              placeholder="e.g. Staging Europe, Production US"
              className="kv-input"
              style={{ width: '100%', marginBottom: '16px' }}
              onKeyDown={(e) => {
                if (e.key === 'Enter' && newEnvName.trim()) {
                  if (onAddEnvironment) onAddEnvironment(newEnvName.trim());
                  setNewEnvName('');
                  setIsNewEnvModalOpen(false);
                }
              }}
            />
            <div style={{ display: 'flex', justifyContent: 'flex-end', gap: '8px' }}>
              <button 
                onClick={() => {
                  setNewEnvName('');
                  setIsNewEnvModalOpen(false);
                }} 
                className="btn-secondary"
              >
                Cancel
              </button>
              <button 
                onClick={() => {
                  if (newEnvName.trim()) {
                    if (onAddEnvironment) onAddEnvironment(newEnvName.trim());
                    setNewEnvName('');
                    setIsNewEnvModalOpen(false);
                  }
                }}
                disabled={!newEnvName.trim()}
                className="btn-send"
              >
                Create Environment
              </button>
            </div>
          </div>
        </div>
      )}

      {/* New Collection Modal */}
      <NewCollectionModal
        isOpen={isNewColModalOpen}
        onClose={() => setIsNewColModalOpen(false)}
        onCreate={onAddCollection}
      />

      {/* Database Connection Modal (Create & Edit) */}
      <NewConnectionModal
        isOpen={isNewDbModalOpen}
        initialConnection={editingDbConfig}
        onClose={() => {
          setIsNewDbModalOpen(false);
          setEditingDbConfig(null);
        }}
        onConnect={(savedDb) => {
          if (editingDbConfig && onUpdateDatabase) {
            onUpdateDatabase(savedDb);
          } else {
            onAddDatabase(savedDb);
          }
          setEditingDbConfig(null);
        }}
      />

      {/* Collections Hub: Import & Export Modal */}
      <ImportExportModal
        isOpen={isImportExportModalOpen}
        onClose={() => setIsImportExportModalOpen(false)}
        collections={collections}
        onImportCollection={(newCol) => {
          if (onImportCollection) {
            onImportCollection(newCol);
          }
        }}
        selectedCollectionId={selectedColForExport}
      />
    </aside>
  );
};
