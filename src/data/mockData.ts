import { Collection, DatabaseConnection, Environment, HistoryItem } from '../types';

export const INITIAL_COLLECTIONS: Collection[] = [];

export const INITIAL_ENVIRONMENTS: Environment[] = [
  {
    id: 'env-default',
    name: 'Default',
    variables: []
  }
];

export const INITIAL_DATABASES: DatabaseConnection[] = [];

export const INITIAL_HISTORY: HistoryItem[] = [];
