import { ApiRequest, Collection, HttpMethod, KeyValuePair } from '../types';

/**
 * Exports a Bapu Studio Collection to Postman Collection v2.1.0 JSON format
 */
export function exportToPostman(collection: Collection): string {
  const postmanItems = collection.requests.map(req => {
    // Headers
    const headers = req.headers
      .filter(h => h.enabled)
      .map(h => ({
        key: h.key,
        value: h.value,
        type: 'text'
      }));

    // Query params
    const query = req.params
      .filter(p => p.enabled)
      .map(p => ({
        key: p.key,
        value: p.value
      }));

    // Body
    let body: any = undefined;
    if (req.bodyType === 'json') {
      body = {
        mode: 'raw',
        raw: req.bodyContent || '{}',
        options: { raw: { language: 'json' } }
      };
    } else if (req.bodyType === 'raw' || req.bodyType === 'form') {
      body = {
        mode: 'raw',
        raw: req.bodyContent || ''
      };
    }

    // Scripts (Events)
    const events: any[] = [];
    if (req.preRequestScript) {
      events.push({
        listen: 'prerequest',
        script: {
          type: 'text/javascript',
          exec: req.preRequestScript.split('\n')
        }
      });
    }
    const testScript = req.testScript || req.tests;
    if (testScript) {
      events.push({
        listen: 'test',
        script: {
          type: 'text/javascript',
          exec: testScript.split('\n')
        }
      });
    }

    // Auth
    let auth: any = undefined;
    if (req.authType === 'bearer' && req.authConfig.token) {
      auth = {
        type: 'bearer',
        bearer: [{ key: 'token', value: req.authConfig.token, type: 'string' }]
      };
    } else if (req.authType === 'basic') {
      auth = {
        type: 'basic',
        basic: [
          { key: 'username', value: req.authConfig.username || '', type: 'string' },
          { key: 'password', value: req.authConfig.password || '', type: 'string' }
        ]
      };
    }

    return {
      name: req.name,
      event: events.length > 0 ? events : undefined,
      request: {
        method: req.method,
        header: headers,
        body: body,
        url: {
          raw: req.url,
          query: query.length > 0 ? query : undefined
        },
        auth: auth
      },
      response: []
    };
  });

  const postmanDoc = {
    info: {
      _postman_id: `bapu-${Date.now()}`,
      name: collection.name,
      description: 'Exported from Bapu Studio',
      schema: 'https://schema.getpostman.com/json/collection/v2.1.0/collection.json'
    },
    item: postmanItems
  };

  return JSON.stringify(postmanDoc, null, 2);
}

/**
 * Exports a Bapu Studio Collection to OpenAPI 3.0.3 Specification
 */
export function exportToOpenApi(collection: Collection): string {
  const paths: Record<string, any> = {};

  collection.requests.forEach(req => {
    let cleanPath = '/';
    try {
      if (req.url.startsWith('http')) {
        const u = new URL(req.url.replace(/\{\{[^}]+\}\}/g, 'var'));
        cleanPath = u.pathname || '/';
      } else {
        cleanPath = req.url.startsWith('/') ? req.url : '/' + req.url;
      }
    } catch {
      cleanPath = req.url.startsWith('/') ? req.url : '/' + req.url;
    }

    if (!paths[cleanPath]) {
      paths[cleanPath] = {};
    }

    const methodKey = req.method.toLowerCase();
    const queryParameters = req.params.map(p => ({
      name: p.key,
      in: 'query',
      required: false,
      schema: { type: 'string', default: p.value }
    }));

    const headerParameters = req.headers.map(h => ({
      name: h.key,
      in: 'header',
      required: false,
      schema: { type: 'string', default: h.value }
    }));

    let requestBody: any = undefined;
    if (['POST', 'PUT', 'PATCH'].includes(req.method) && req.bodyContent) {
      requestBody = {
        content: {
          'application/json': {
            schema: { type: 'object' },
            example: req.bodyContent
          }
        }
      };
    }

    paths[cleanPath][methodKey] = {
      summary: req.name,
      parameters: [...queryParameters, ...headerParameters],
      requestBody: requestBody,
      responses: {
        '200': {
          description: 'Successful execution',
          content: {
            'application/json': {
              schema: { type: 'object' }
            }
          }
        }
      }
    };
  });

  const openApiDoc = {
    openapi: '3.0.3',
    info: {
      title: collection.name,
      version: '1.0.0',
      description: 'Exported from Bapu Studio'
    },
    paths: paths
  };

  return JSON.stringify(openApiDoc, null, 2);
}

/**
 * Imports collections from Postman, OpenAPI, or native Bapu format
 */
export function importCollection(rawInput: string, customName?: string): Collection {
  const parsed = typeof rawInput === 'string' ? JSON.parse(rawInput) : rawInput;

  // Case 1: Native Bapu Collection
  if (parsed.id && parsed.name && Array.isArray(parsed.requests)) {
    return {
      ...parsed,
      id: `col-${Date.now()}`,
      name: customName || parsed.name
    };
  }

  // Case 2: Postman Collection (v2.0 or v2.1)
  if (parsed.info && (parsed.item || parsed.items)) {
    const colName = customName || parsed.info.name || 'Imported Postman Collection';
    const items = parsed.item || parsed.items || [];
    const requests: ApiRequest[] = [];

    function extractPostmanItems(itemList: any[]) {
      itemList.forEach(item => {
        if (item.request) {
          // It's a request
          const reqObj = item.request;
          const method: HttpMethod = (typeof reqObj === 'string' ? 'GET' : reqObj.method || 'GET').toUpperCase() as HttpMethod;
          
          let url = '';
          const params: KeyValuePair[] = [];
          if (typeof reqObj === 'string') {
            url = reqObj;
          } else if (typeof reqObj.url === 'string') {
            url = reqObj.url;
          } else if (reqObj.url) {
            url = reqObj.url.raw || '';
            if (Array.isArray(reqObj.url.query)) {
              reqObj.url.query.forEach((q: any) => {
                params.push({
                  id: `p-${Date.now()}-${Math.random()}`,
                  key: q.key || '',
                  value: q.value || '',
                  enabled: q.disabled !== true
                });
              });
            }
          }

          const headers: KeyValuePair[] = [];
          if (Array.isArray(reqObj.header)) {
            reqObj.header.forEach((h: any) => {
              headers.push({
                id: `h-${Date.now()}-${Math.random()}`,
                key: h.key || '',
                value: h.value || '',
                enabled: h.disabled !== true
              });
            });
          }

          let bodyType: 'none' | 'json' | 'form' | 'raw' = 'none';
          let bodyContent = '';
          if (reqObj.body) {
            if (reqObj.body.mode === 'raw') {
              bodyContent = reqObj.body.raw || '';
              bodyType = bodyContent.trim().startsWith('{') || bodyContent.trim().startsWith('[') ? 'json' : 'raw';
            } else if (reqObj.body.mode === 'urlencoded' || reqObj.body.mode === 'formdata') {
              bodyType = 'form';
              const formPairs = reqObj.body.urlencoded || reqObj.body.formdata || [];
              bodyContent = JSON.stringify(formPairs, null, 2);
            }
          }

          // Extract auth
          let authType: 'none' | 'bearer' | 'basic' | 'apikey' = 'none';
          const authConfig: any = {};
          if (reqObj.auth) {
            if (reqObj.auth.type === 'bearer' && reqObj.auth.bearer) {
              authType = 'bearer';
              const tok = reqObj.auth.bearer.find((b: any) => b.key === 'token');
              authConfig.token = tok ? tok.value : '';
            } else if (reqObj.auth.type === 'basic' && reqObj.auth.basic) {
              authType = 'basic';
              const u = reqObj.auth.basic.find((b: any) => b.key === 'username');
              const p = reqObj.auth.basic.find((b: any) => b.key === 'password');
              authConfig.username = u ? u.value : '';
              authConfig.password = p ? p.value : '';
            }
          }

          // Extract Pre-request & Test scripts
          let preRequestScript = '';
          let testScript = '';
          if (Array.isArray(item.event)) {
            const pre = item.event.find((e: any) => e.listen === 'prerequest');
            if (pre && pre.script && Array.isArray(pre.script.exec)) {
              preRequestScript = pre.script.exec.join('\n');
            }
            const test = item.event.find((e: any) => e.listen === 'test');
            if (test && test.script && Array.isArray(test.script.exec)) {
              testScript = test.script.exec.join('\n');
            }
          }

          requests.push({
            id: `req-${Date.now()}-${Math.random()}`,
            name: item.name || `${method} Request`,
            method: method,
            url: url,
            params: params,
            headers: headers,
            bodyType: bodyType,
            bodyContent: bodyContent,
            authType: authType,
            authConfig: authConfig,
            preRequestScript: preRequestScript,
            testScript: testScript,
            tests: testScript
          });
        } else if (item.item && Array.isArray(item.item)) {
          // Folder: recurse
          extractPostmanItems(item.item);
        }
      });
    }

    extractPostmanItems(items);

    return {
      id: `col-${Date.now()}`,
      name: colName,
      requests: requests
    };
  }

  // Case 3: OpenAPI / Swagger Specification (v2.0, v3.0, v3.1)
  if (parsed.openapi || parsed.swagger || parsed.paths) {
    const colName = customName || (parsed.info && parsed.info.title) || 'Imported OpenAPI Collection';
    const requests: ApiRequest[] = [];
    const paths = parsed.paths || {};

    const baseUrl = (parsed.servers && parsed.servers[0] && parsed.servers[0].url) || 'https://api.example.com';

    Object.entries(paths).forEach(([pathKey, pathObj]: [string, any]) => {
      const methods = ['get', 'post', 'put', 'delete', 'patch', 'options', 'head'];
      methods.forEach(m => {
        if (pathObj[m]) {
          const op = pathObj[m];
          const method = m.toUpperCase() as HttpMethod;
          const params: KeyValuePair[] = [];
          const headers: KeyValuePair[] = [];

          if (Array.isArray(op.parameters)) {
            op.parameters.forEach((p: any) => {
              if (p.in === 'query') {
                params.push({
                  id: `p-${Date.now()}-${Math.random()}`,
                  key: p.name,
                  value: p.example || (p.schema && p.schema.default) || '',
                  enabled: true
                });
              } else if (p.in === 'header') {
                headers.push({
                  id: `h-${Date.now()}-${Math.random()}`,
                  key: p.name,
                  value: p.example || '',
                  enabled: true
                });
              }
            });
          }

          let bodyContent = '';
          let bodyType: 'none' | 'json' | 'form' | 'raw' = 'none';
          if (op.requestBody && op.requestBody.content) {
            bodyType = 'json';
            const jsonContent = op.requestBody.content['application/json'];
            if (jsonContent) {
              if (jsonContent.example) {
                bodyContent = typeof jsonContent.example === 'object' ? JSON.stringify(jsonContent.example, null, 2) : jsonContent.example;
              } else if (jsonContent.schema && jsonContent.schema.example) {
                bodyContent = JSON.stringify(jsonContent.schema.example, null, 2);
              } else {
                bodyContent = '{\n  "example": true\n}';
              }
            }
          }

          requests.push({
            id: `req-${Date.now()}-${Math.random()}`,
            name: op.summary || op.operationId || `${method} ${pathKey}`,
            method: method,
            url: baseUrl.replace(/\/$/, '') + pathKey,
            params: params,
            headers: headers,
            bodyType: bodyType,
            bodyContent: bodyContent,
            authType: 'none',
            authConfig: {}
          });
        }
      });
    });

    return {
      id: `col-${Date.now()}`,
      name: colName,
      requests: requests
    };
  }

  throw new Error('Unrecognized collection format. Please provide a valid Postman Collection v2.1, OpenAPI 3.0, or Bapu JSON file.');
}
