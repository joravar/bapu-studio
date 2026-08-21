import { ApiRequest, Environment } from '../types';

export type SupportedLanguage = 
  | 'javascript_fetch' 
  | 'javascript_axios' 
  | 'python_requests' 
  | 'python_httpx' 
  | 'go_http' 
  | 'rust_reqwest' 
  | 'shell_curl';

export function generateCodeSnippet(req: ApiRequest, env: Environment, lang: SupportedLanguage): string {
  // Resolve environment variables
  let resolvedUrl = req.url;
  env.variables.forEach(v => {
    if (v.enabled) {
      resolvedUrl = resolvedUrl.replaceAll(`{{${v.key}}}`, v.value);
    }
  });

  // Append enabled query params
  const activeParams = req.params.filter(p => p.enabled && p.key);
  if (activeParams.length > 0) {
    const qs = activeParams.map(p => `${encodeURIComponent(p.key)}=${encodeURIComponent(p.value)}`).join('&');
    resolvedUrl += (resolvedUrl.includes('?') ? '&' : '?') + qs;
  }

  // Active headers
  const activeHeaders = req.headers.filter(h => h.enabled && h.key);
  const headerObj: Record<string, string> = {};
  activeHeaders.forEach(h => {
    let val = h.value;
    env.variables.forEach(v => {
      if (v.enabled) val = val.replaceAll(`{{${v.key}}}`, v.value);
    });
    headerObj[h.key] = val;
  });

  // Calculate effective body (GraphQL or standard body content)
  let effectiveBody = req.bodyContent;
  if (req.bodyType === 'graphql') {
    let parsedVars = {};
    if (req.graphqlVariables) {
      try { parsedVars = JSON.parse(req.graphqlVariables); } catch {}
    }
    effectiveBody = JSON.stringify({
      query: req.graphqlQuery || '',
      variables: parsedVars
    }, null, 2);

    if (!Object.keys(headerObj).some(k => k.toLowerCase() === 'content-type')) {
      headerObj['Content-Type'] = 'application/json';
    }
  }

  const hasBody = Boolean(effectiveBody && req.method !== 'GET');

  switch (lang) {
    case 'javascript_fetch':
      return `// JavaScript / TypeScript (fetch)
const url = "${resolvedUrl}";
const options = {
  method: "${req.method}",
  headers: ${JSON.stringify(headerObj, null, 2)},
${hasBody ? `  body: JSON.stringify(${effectiveBody})` : ''}
};

try {
  const response = await fetch(url, options);
  const data = await response.json();
  console.log(data);
} catch (error) {
  console.error("Request failed:", error);
}`;

    case 'javascript_axios':
      return `// JavaScript / TypeScript (axios)
import axios from 'axios';

const config = {
  method: '${req.method.toLowerCase()}',
  url: '${resolvedUrl}',
  headers: ${JSON.stringify(headerObj, null, 2)},
${hasBody ? `  data: ${effectiveBody}` : ''}
};

axios(config)
  .then((response) => console.log(JSON.stringify(response.data)))
  .catch((error) => console.error(error));`;

    case 'python_requests':
      return `# Python (requests)
import requests
import json

url = "${resolvedUrl}"
headers = ${JSON.stringify(headerObj, null, 4).replace(/"/g, "'")}
${hasBody ? `payload = ${effectiveBody}` : 'payload = None'}

response = requests.request(
    "${req.method}",
    url,
    headers=headers,
    ${hasBody ? 'json=payload' : ''}
)

print(response.status_code)
print(response.json())`;

    case 'python_httpx':
      return `# Python Async (httpx)
import httpx
import asyncio

async def main():
    url = "${resolvedUrl}"
    headers = ${JSON.stringify(headerObj, null, 4).replace(/"/g, "'")}
    ${hasBody ? `payload = ${effectiveBody}` : ''}

    async with httpx.AsyncClient() as client:
        response = await client.request(
            "${req.method}",
            url,
            headers=headers,
            ${hasBody ? 'json=payload' : ''}
        )
        print(response.json())

asyncio.run(main())`;

    case 'go_http':
      return `// Go (net/http)
package main

import (
	"fmt"
	"net/http"
	"io"
	"strings"
)

func main() {
	url := "${resolvedUrl}"
	method := "${req.method}"

	${hasBody ? `payload := strings.NewReader(\`${effectiveBody}\`)` : `payload := nil`}

	client := &http.Client{}
	req, err := http.NewRequest(method, url, payload)
	if err != nil {
		panic(err)
	}

${Object.entries(headerObj).map(([k, v]) => `\treq.Header.Add("${k}", "${v}")`).join('\n')}

	res, err := client.Do(req)
	if err != nil {
		panic(err)
	}
	defer res.Body.Close()

	body, err := io.ReadAll(res.Body)
	fmt.Println(string(body))
}`;

    case 'rust_reqwest':
      return `// Rust (reqwest + tokio)
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use std::error::Error;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let client = reqwest::Client::new();
    let mut headers = HeaderMap::new();

${Object.entries(headerObj).map(([k, v]) => `    headers.insert(HeaderName::from_static("${k.toLowerCase()}"), HeaderValue::from_static("${v}"));`).join('\n')}

    let response = client
        .${req.method.toLowerCase()}("${resolvedUrl}")
        .headers(headers)
${hasBody ? `        .body(r#"${effectiveBody}"#)` : ''}
        .send()
        .await?;

    let body = response.text().await?;
    println!("{:#?}", body);
    Ok(())
}`;

    case 'shell_curl':
    default: {
      let cmd = `curl -X ${req.method} "${resolvedUrl}"`;
      Object.entries(headerObj).forEach(([k, v]) => {
        cmd += ` \\\n  -H "${k}: ${v}"`;
      });
      if (hasBody) {
        cmd += ` \\\n  -d '${effectiveBody.replace(/'/g, "'\\''")}'`;
      }
      return cmd;
    }
  }
}
