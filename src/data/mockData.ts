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
  name: 'Local Analytics (Playground SQL)',
  type: 'postgres',
  database: 'saas_production_db',
  isConnected: true,
  isDemoDb: true,
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

export const SAMPLE_MONGODB_PLAYGROUND_DB: DatabaseConnection = {
  id: 'db-playground-mongodb',
  name: 'Demo MongoDB (E-Commerce)',
  type: 'mongodb',
  database: 'store_inventory',
  isConnected: true,
  isDemoDb: true,
  tables: [
    {
      name: 'products',
      rowCount: 320,
      columns: [
        { name: '_id', type: 'OBJECTID', isPrimaryKey: true, isNullable: false },
        { name: 'name', type: 'STRING', isPrimaryKey: false, isNullable: false },
        { name: 'sku', type: 'STRING', isPrimaryKey: false, isNullable: false },
        { name: 'price', type: 'DOUBLE', isPrimaryKey: false, isNullable: false },
        { name: 'inStock', type: 'BOOLEAN', isPrimaryKey: false, isNullable: false },
        { name: 'category', type: 'STRING', isPrimaryKey: false, isNullable: false },
        { name: 'tags', type: 'ARRAY', isPrimaryKey: false, isNullable: true }
      ]
    },
    {
      name: 'orders',
      rowCount: 145,
      columns: [
        { name: '_id', type: 'OBJECTID', isPrimaryKey: true, isNullable: false },
        { name: 'customer_email', type: 'STRING', isPrimaryKey: false, isNullable: false },
        { name: 'total', type: 'DOUBLE', isPrimaryKey: false, isNullable: false },
        { name: 'status', type: 'STRING', isPrimaryKey: false, isNullable: false },
        { name: 'items_count', type: 'INT', isPrimaryKey: false, isNullable: false }
      ]
    }
  ]
};

export const INITIAL_DATABASES: DatabaseConnection[] = [];

export const INITIAL_HISTORY: HistoryItem[] = [];
