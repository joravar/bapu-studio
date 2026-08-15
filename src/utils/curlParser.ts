import { ApiRequest, HttpMethod, KeyValuePair } from '../types';

export function parseCurlCommand(rawCurl: string): Partial<ApiRequest> {
  const cleanCmd = rawCurl.trim().replace(/\\\r?\n/g, ' ');
  
  let method: HttpMethod = 'GET';
  let url = '';
  const headers: KeyValuePair[] = [];
  const params: KeyValuePair[] = [];
  let bodyContent = '';
  let bodyType: 'none' | 'json' | 'form' | 'raw' = 'none';

  // Extract URL (first token that starts with http or the first positional argument)
  const urlMatch = cleanCmd.match(/(?:['"])(https?:\/\/[^'"]+)(?:['"])|(?:https?:\/\/[^\s]+)/i);
  if (urlMatch) {
    const fullUrl = urlMatch[1] || urlMatch[0];
    try {
      const parsedUrl = new URL(fullUrl);
      url = `${parsedUrl.origin}${parsedUrl.pathname}`;
      parsedUrl.searchParams.forEach((val, key) => {
        params.push({
          id: `p-${Date.now()}-${Math.random().toString(36).substr(2, 4)}`,
          key,
          value: val,
          enabled: true
        });
      });
    } catch {
      url = fullUrl;
    }
  }

  // Extract Method (-X POST / --request POST)
  const methodMatch = cleanCmd.match(/(?:-X|--request)\s+([A-Z]+)/i);
  if (methodMatch) {
    method = methodMatch[1].toUpperCase() as HttpMethod;
  }

  // Extract Headers (-H "Key: Value" / --header 'Key: Value')
  const headerRegex = /(?:-H|--header)\s+["']([^"']+)["']/gi;
  let hMatch;
  while ((hMatch = headerRegex.exec(cleanCmd)) !== null) {
    const headerStr = hMatch[1];
    const colonIdx = headerStr.indexOf(':');
    if (colonIdx > 0) {
      const key = headerStr.slice(0, colonIdx).trim();
      const value = headerStr.slice(colonIdx + 1).trim();
      headers.push({
        id: `h-${Date.now()}-${Math.random().toString(36).substr(2, 4)}`,
        key,
        value,
        enabled: true
      });
    }
  }

  // Extract Body (-d '...' / --data '...' / --data-raw '...')
  const dataRegex = /(?:-d|--data|--data-raw|--data-binary)\s+(['"])([\s\S]*?)\1/i;
  const dataMatch = cleanCmd.match(dataRegex);
  if (dataMatch) {
    bodyContent = dataMatch[2];
    bodyType = 'raw';
    if (!methodMatch) method = 'POST';

    try {
      JSON.parse(bodyContent);
      bodyType = 'json';
    } catch {
      // not JSON
    }
  }

  return {
    name: `Imported from cURL (${method})`,
    method,
    url: url || 'https://api.example.com/v1/resource',
    params,
    headers,
    bodyType,
    bodyContent,
    authType: 'none',
    authConfig: {}
  };
}
