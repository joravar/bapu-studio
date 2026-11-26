//! Zero-latency local schema analyzer that synthesizes comprehensive `bapu.test(...)`
//! assertions directly from any live `ApiResponse` (no LLM or cloud call required).

use crate::types::ApiResponse;

pub fn generate_assertions_from_response(res: &ApiResponse) -> String {
    let mut lines: Vec<String> = Vec::new();
    lines.push("// Auto-generated response assertions by Bapu Studio".into());
    lines.push("const data = bapu.response.json();\n".into());

    // 1. Status code
    lines.push(format!("bapu.test(\"Status code is {}\", () => {{", res.status));
    lines.push(format!("  bapu.expect(bapu.response.status).toBe({});", res.status));
    lines.push("});\n".into());

    // 2. Latency budget
    let max_ms = ((res.time_ms as f64) * 2.5).ceil() as u64;
    let budget = max_ms.max(1000);
    lines.push(format!("bapu.test(\"Response time is under {budget}ms\", () => {{"));
    lines.push(format!("  bapu.expect(bapu.response.timeMs).toBeLessThan({budget});"));
    lines.push("});\n".into());

    // 3. Content-Type header check
    let ct = res
        .headers
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case("content-type"))
        .map(|(_, v)| v.as_str())
        .unwrap_or("");
    if ct.contains("json") {
        lines.push("bapu.test(\"Content-Type is JSON\", () => {".into());
        lines.push("  bapu.response.to.be.json();".into());
        lines.push("});\n".into());
    }

    // 4. Schema-level payload checks
    match &res.data {
        serde_json::Value::Array(arr) => {
            lines.push("bapu.test(\"Response payload is an array\", () => {".into());
            lines.push("  bapu.expect(Array.isArray(data)).toBe(true);".into());
            if !arr.is_empty() {
                lines.push("  bapu.expect(data.length).toBeGreaterThan(0);".into());
            }
            lines.push("});\n".into());

            if let Some(serde_json::Value::Object(sample)) = arr.first() {
                lines.push("bapu.test(\"Array items match expected schema\", () => {".into());
                lines.push("  const item = data[0];".into());
                for (k, v) in sample.iter().take(6) {
                    append_field_assertion(&mut lines, "item", k, v);
                }
                lines.push("});\n".into());
            }
        }
        serde_json::Value::Object(obj) if !obj.is_empty() => {
            lines.push("bapu.test(\"Response object contains required schema fields\", () => {".into());
            for (k, v) in obj.iter().take(8) {
                append_field_assertion(&mut lines, "data", k, v);
            }
            lines.push("});\n".into());
        }
        _ => {}
    }

    lines.join("\n")
}

fn append_field_assertion(lines: &mut Vec<String>, target: &str, key: &str, val: &serde_json::Value) {
    match val {
        serde_json::Value::Null => {
            lines.push(format!("  bapu.expect(\"{key}\" in {target}).toBe(true);"));
        }
        serde_json::Value::String(s) => {
            lines.push(format!("  bapu.expect(typeof {target}[\"{key}\"]).toBe(\"string\");"));
            if s.contains('@') && s.contains('.') {
                lines.push(format!("  bapu.expect({target}[\"{key}\"]).toContain(\"@\");"));
            } else if s.starts_with("http://") || s.starts_with("https://") {
                lines.push(format!("  bapu.expect({target}[\"{key}\"]).toMatch(/^https?:\\/\\//);"));
            }
        }
        serde_json::Value::Number(_) => {
            lines.push(format!("  bapu.expect(typeof {target}[\"{key}\"]).toBe(\"number\");"));
        }
        serde_json::Value::Bool(_) => {
            lines.push(format!("  bapu.expect(typeof {target}[\"{key}\"]).toBe(\"boolean\");"));
        }
        serde_json::Value::Array(_) => {
            lines.push(format!("  bapu.expect(Array.isArray({target}[\"{key}\"])).toBe(true);"));
        }
        serde_json::Value::Object(_) => {
            lines.push(format!("  bapu.expect(typeof {target}[\"{key}\"]).toBe(\"object\");"));
        }
    }
}
