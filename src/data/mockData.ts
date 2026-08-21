import { Collection, DatabaseConnection, Environment, HistoryItem } from '../types';

export const INITIAL_COLLECTIONS: Collection[] = [];

export const INITIAL_ENVIRONMENTS: Environment[] = [
  {
    id: 'env-default',
    name: 'Default',
    variables: []
  }
];

export const SAMPLE_PLAYGROUND_DB: DatabaseConnection = {
  id: 'db-playground-analytics',
  name: 'Local Analytics (Playground)',
  type: 'postgres',
  database: 'saas_production_db',
  isConnected: true,
  tables: [
    {
      name: 'users',
      rowCount: 1420,
      columns: [
        { name: 'id', type: 'INT', isPrimaryKey: true, isNullable: false },
        { name: 'email', type: 'VARCHAR(255)', isPrimaryKey: false, isNullable: false },
        { name: 'role', type: 'VARCHAR(50)', isPrimaryKey: false, isNullable: false },
        { name: 'status', type: 'VARCHAR(50)', isPrimaryKey: false, isNullable: true },
        { name: 'created_at', type: 'TIMESTAMPTZ', isPrimaryKey: false, isNullable: false }
      ]
    },
    {
      name: 'workspaces',
      rowCount: 480,
      columns: [
        { name: 'id', type: 'VARCHAR(50)', isPrimaryKey: true, isNullable: false },
        { name: 'name', type: 'VARCHAR(255)', isPrimaryKey: false, isNullable: false },
        { name: 'plan_tier', type: 'VARCHAR(50)', isPrimaryKey: false, isNullable: false },
        { name: 'storage_mb', type: 'INT', isPrimaryKey: false, isNullable: true },
        { name: 'created_at', type: 'TIMESTAMPTZ', isPrimaryKey: false, isNullable: false }
      ]
    },
    {
      name: 'accounts',
      rowCount: 850,
      columns: [
        { name: 'id', type: 'VARCHAR(50)', isPrimaryKey: true, isNullable: false },
        { name: 'user_id', type: 'VARCHAR(50)', isPrimaryKey: false, isNullable: false },
        { name: 'balance_cents', type: 'BIGINT', isPrimaryKey: false, isNullable: true },
        { name: 'currency', type: 'VARCHAR(10)', isPrimaryKey: false, isNullable: false },
        { name: 'status', type: 'VARCHAR(50)', isPrimaryKey: false, isNullable: true }
      ]
    }
  ]
};

export const INITIAL_DATABASES: DatabaseConnection[] = [];

export const INITIAL_HISTORY: HistoryItem[] = [];
