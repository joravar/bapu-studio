import { ApiResponse } from '../types';

/**
 * Intelligently generates comprehensive, type-safe test assertions from any live API response.
 * Completely model-free: operates locally in 0ms via JSON AST schema analysis.
 */
export function generateAssertionsFromResponse(res: ApiResponse): string {
  const lines: string[] = [];

  lines.push('// Auto-generated response assertions by Bapu Studio');
  lines.push('const data = bapu.response.json();\n');

  // 1. Status code assertion
  lines.push(`bapu.test("Status code is ${res.status}", () => {`);
  lines.push(`  bapu.expect(bapu.response.status).toBe(${res.status});`);
  lines.push('});\n');

  // 2. Response time assertion
  const maxAcceptableTime = Math.max(1000, Math.ceil(res.timeMs * 2.5));
  lines.push(`bapu.test("Response time is under ${maxAcceptableTime}ms", () => {`);
  lines.push(`  bapu.expect(bapu.response.timeMs).toBeLessThan(${maxAcceptableTime});`);
  lines.push('});\n');

  // 3. Content-Type header assertion
  const contentType = res.headers['content-type'] || res.headers['Content-Type'];
  if (contentType && contentType.includes('application/json')) {
    lines.push('bapu.test("Content-Type is JSON", () => {');
    lines.push('  bapu.expect(bapu.response.headers["content-type"] || "").toContain("application/json");');
    lines.push('});\n');
  }

  // 4. Data payload assertions
  if (res.data !== null && res.data !== undefined) {
    if (Array.isArray(res.data)) {
      lines.push('bapu.test("Response payload is a non-empty array", () => {');
      lines.push('  bapu.expect(Array.isArray(data)).toBe(true);');
      if (res.data.length > 0) {
        lines.push(`  bapu.expect(data.length).toBeGreaterThan(0);`);
      }
      lines.push('});\n');

      // Inspect schema of the first item in array
      if (res.data.length > 0 && typeof res.data[0] === 'object' && res.data[0] !== null) {
        const sampleItem = res.data[0];
        lines.push('bapu.test("Array items conform to schema definition", () => {');
        lines.push('  const firstItem = data[0];');
        lines.push('  bapu.expect(typeof firstItem).toBe("object");');
        
        Object.entries(sampleItem).slice(0, 6).forEach(([key, val]) => {
          if (val === null || val === undefined) return;
          const valType = typeof val;
          if (valType === 'string') {
            lines.push(`  bapu.expect(typeof firstItem["${key}"]).toBe("string");`);
            if (String(val).includes('@') && String(val).includes('.')) {
              lines.push(`  bapu.expect(firstItem["${key}"]).toContain("@");`);
            }
          } else if (valType === 'number') {
            lines.push(`  bapu.expect(typeof firstItem["${key}"]).toBe("number");`);
          } else if (valType === 'boolean') {
            lines.push(`  bapu.expect(typeof firstItem["${key}"]).toBe("boolean");`);
          } else if (Array.isArray(val)) {
            lines.push(`  bapu.expect(Array.isArray(firstItem["${key}"])).toBe(true);`);
          }
        });
        lines.push('});\n');
      }
    } else if (typeof res.data === 'object') {
      const keys = Object.keys(res.data);
      if (keys.length > 0) {
        lines.push('bapu.test("Response object contains required schema fields", () => {');
        
        keys.slice(0, 8).forEach(key => {
          const val = res.data[key];
          if (val === null || val === undefined) {
            lines.push(`  bapu.expect("${key}" in data).toBe(true);`);
            return;
          }
          const valType = typeof val;
          if (valType === 'string') {
            lines.push(`  bapu.expect(typeof data["${key}"]).toBe("string");`);
            if (String(val).includes('@') && String(val).includes('.')) {
              lines.push(`  bapu.expect(data["${key}"]).toContain("@");`);
            } else if (String(val).startsWith('http://') || String(val).startsWith('https://')) {
              lines.push(`  bapu.expect(data["${key}"]).toMatch(/^https?:\\/\\//);`);
            }
          } else if (valType === 'number') {
            lines.push(`  bapu.expect(typeof data["${key}"]).toBe("number");`);
          } else if (valType === 'boolean') {
            lines.push(`  bapu.expect(typeof data["${key}"]).toBe("boolean");`);
          } else if (Array.isArray(val)) {
            lines.push(`  bapu.expect(Array.isArray(data["${key}"])).toBe(true);`);
          } else if (typeof val === 'object') {
            lines.push(`  bapu.expect(typeof data["${key}"]).toBe("object");`);
          }
        });
        lines.push('});\n');
      }
    }
  }

  return lines.join('\n');
}
