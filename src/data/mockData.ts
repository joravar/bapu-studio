import { Collection, DatabaseConnection, Environment, HistoryItem } from '../types';

export const INITIAL_COLLECTIONS: Collection[] = [
  {
    id: 'col-auth',
    name: 'Authentication & Users',
    requests: [
      {
        id: 'req-1',
        name: 'Get Current User Profile',
        method: 'GET',
        url: '{{API_BASE_URL}}/v1/users/me',
        params: [{ id: 'p1', key: 'expand', value: 'permissions,teams', enabled: true }],
        headers: [
          { id: 'h1', key: 'Accept', value: 'application/json', enabled: true },
          { id: 'h2', key: 'Authorization', value: 'Bearer {{AUTH_TOKEN}}', enabled: true }
        ],
        bodyType: 'none',
        bodyContent: '',
        authType: 'bearer',
        authConfig: { token: '{{AUTH_TOKEN}}' }
      },
      {
        id: 'req-2',
        name: 'Generate API Secret Key',
        method: 'POST',
        url: '{{API_BASE_URL}}/v1/api-keys/generate',
        params: [],
        headers: [
          { id: 'h3', key: 'Content-Type', value: 'application/json', enabled: true }
        ],
        bodyType: 'json',
        bodyContent: JSON.stringify({
          name: "production-deploy-key",
          scopes: ["read:data", "write:deployments"],
          expiresInDays: 90
        }, null, 2),
        authType: 'bearer',
        authConfig: { token: '{{AUTH_TOKEN}}' }
      }
    ]
  },
  {
    id: 'col-billing',
    name: 'Billing & Subscriptions (Demo API)',
    requests: [
      {
        id: 'req-3',
        name: 'List Active Subscriptions',
        method: 'GET',
        url: 'https://api.example.com/v1/subscriptions',
        params: [
          { id: 'p2', key: 'status', value: 'active', enabled: true },
          { id: 'p3', key: 'limit', value: '10', enabled: true }
        ],
        headers: [
          { id: 'h4', key: 'Authorization', value: 'Bearer {{API_SECRET_KEY}}', enabled: true }
        ],
        bodyType: 'none',
        bodyContent: '',
        authType: 'bearer',
        authConfig: { token: '{{API_SECRET_KEY}}' }
      },
      {
        id: 'req-4',
        name: 'Create Checkout Session',
        method: 'POST',
        url: 'https://api.example.com/v1/checkout/sessions',
        params: [],
        headers: [
          { id: 'h5', key: 'Content-Type', value: 'application/json', enabled: true }
        ],
        bodyType: 'json',
        bodyContent: JSON.stringify({
          customer_email: "dev@example.org",
          line_items: [{ plan: "pro_tier_yearly", quantity: 1 }],
          mode: "subscription",
          success_url: "https://example.org/success"
        }, null, 2),
        authType: 'bearer',
        authConfig: { token: '{{API_SECRET_KEY}}' }
      }
    ]
  }
];

export const INITIAL_ENVIRONMENTS: Environment[] = [
  {
    id: 'env-local',
    name: 'Local Development',
    variables: [
      { id: 'v1', key: 'API_BASE_URL', value: 'http://localhost:8080', enabled: true, isSecret: false },
      { id: 'v2', key: 'AUTH_TOKEN', value: 'jwt_local_dev_secret_token_123', enabled: true, isSecret: true },
      { id: 'v3', key: 'API_SECRET_KEY', value: 'sec_test_demo_key_example_99182', enabled: true, isSecret: true },
      { id: 'v4', key: 'DB_PORT', value: '5432', enabled: true, isSecret: false }
    ]
  },
  {
    id: 'env-prod',
    name: 'Production Cloud',
    variables: [
      { id: 'v5', key: 'API_BASE_URL', value: 'https://api.example.com', enabled: true, isSecret: false },
      { id: 'v6', key: 'AUTH_TOKEN', value: 'jwt_live_prod_secret_token_999', enabled: true, isSecret: true },
      { id: 'v7', key: 'API_SECRET_KEY', value: 'sec_live_prod_key_example_38819', enabled: true, isSecret: true },
      { id: 'v8', key: 'DB_PORT', value: '6543', enabled: true, isSecret: false }
    ]
  }
];

export const INITIAL_DATABASES: DatabaseConnection[] = [
  {
    id: 'db-main',
    name: 'Production Postgres (Neon)',
    type: 'postgres',
    database: 'nexus_core_db',
    isConnected: true,
    tables: [
      {
        name: 'users',
        rowCount: 14208,
        columns: [
          { name: 'id', type: 'UUID', isPrimaryKey: true, isNullable: false },
          { name: 'email', type: 'VARCHAR(255)', isPrimaryKey: false, isNullable: false },
          { name: 'organization_id', type: 'UUID', isPrimaryKey: false, isNullable: true },
          { name: 'role', type: 'VARCHAR(50)', isPrimaryKey: false, isNullable: false },
          { name: 'created_at', type: 'TIMESTAMPTZ', isPrimaryKey: false, isNullable: false }
        ]
      },
      {
        name: 'api_tokens',
        rowCount: 8520,
        columns: [
          { name: 'id', type: 'UUID', isPrimaryKey: true, isNullable: false },
          { name: 'user_id', type: 'UUID', isPrimaryKey: false, isNullable: false },
          { name: 'name', type: 'VARCHAR(100)', isPrimaryKey: false, isNullable: false },
          { name: 'token_hash', type: 'TEXT', isPrimaryKey: false, isNullable: false },
          { name: 'last_used_at', type: 'TIMESTAMPTZ', isPrimaryKey: false, isNullable: true }
        ]
      },
      {
        name: 'workspaces',
        rowCount: 3120,
        columns: [
          { name: 'id', type: 'UUID', isPrimaryKey: true, isNullable: false },
          { name: 'name', type: 'VARCHAR(120)', isPrimaryKey: false, isNullable: false },
          { name: 'plan_tier', type: 'VARCHAR(50)', isPrimaryKey: false, isNullable: false },
          { name: 'storage_mb', type: 'INT', isPrimaryKey: false, isNullable: false }
        ]
      }
    ]
  },
  {
    id: 'db-cache',
    name: 'Redis Cache Cluster',
    type: 'redis',
    database: '0',
    isConnected: true,
    tables: [
      {
        name: 'session:tokens',
        rowCount: 45100,
        columns: [
          { name: 'key', type: 'STRING', isPrimaryKey: true, isNullable: false },
          { name: 'ttl_seconds', type: 'INTEGER', isPrimaryKey: false, isNullable: false }
        ]
      }
    ]
  }
];

export const INITIAL_HISTORY: HistoryItem[] = [
  {
    id: 'hist-1',
    type: 'api',
    title: 'GET /v1/users/me',
    subtitle: '200 OK • 32ms',
    status: 200,
    timestamp: '2 mins ago'
  },
  {
    id: 'hist-2',
    type: 'sql',
    title: 'SELECT * FROM users LIMIT 10;',
    subtitle: '10 rows • 14ms',
    timestamp: '5 mins ago'
  },
  {
    id: 'hist-3',
    type: 'api',
    title: 'POST /v1/api-keys/generate',
    subtitle: '201 Created • 88ms',
    status: 201,
    timestamp: '12 mins ago'
  }
];
