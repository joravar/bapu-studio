export type HttpMethod = 'GET' | 'POST' | 'PUT' | 'PATCH' | 'DELETE';

export interface KeyValuePair {
  id: string;
  key: string;
  value: string;
  enabled: boolean;
  isSecret?: boolean;
}

export interface ApiRequest {
  id: string;
  name: string;
  method: HttpMethod;
  url: string;
  params: KeyValuePair[];
  headers: KeyValuePair[];
  bodyType: 'none' | 'json' | 'form' | 'raw';
  bodyContent: string;
  authType: 'none' | 'bearer' | 'basic' | 'apikey';
  authConfig: {
    token?: string;
    username?: string;
    password?: string;
    apiKeyName?: string;
    apiKeyValue?: string;
    addTo?: 'header' | 'query';
  };
  collectionId?: string;
}

export interface ApiResponse {
  status: number;
  statusText: string;
  timeMs: number;
  sizeBytes: number;
  headers: Record<string, string>;
  data: any;
  timestamp: string;
}

export interface ColumnDefinition {
  name: string;
  type: string;
  isPrimaryKey: boolean;
  isNullable: boolean;
}

export interface TableSchema {
  name: string;
  rowCount: number;
  columns: ColumnDefinition[];
}

export interface DatabaseConnection {
  id: string;
  name: string;
  type: 'sqlite' | 'postgres' | 'mysql' | 'redis' | 'mongodb';
  connectionString?: string;
  database: string;
  isConnected: boolean;
  tables: TableSchema[];
}

export interface QueryResult {
  columns: string[];
  rows: Record<string, any>[];
  rowCount: number;
  executionTimeMs: number;
  error?: string;
}

export interface Environment {
  id: string;
  name: string;
  variables: KeyValuePair[];
}

export interface HistoryItem {
  id: string;
  type: 'api' | 'sql';
  title: string;
  subtitle: string;
  status?: number;
  timestamp: string;
}

export interface Collection {
  id: string;
  name: string;
  requests: ApiRequest[];
}
