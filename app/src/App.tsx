import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import "./App.css";

interface Finding {
  start: number;
  end: number;
  rule_id: string;
  matched: string;
}

function App() {
  const [text, setText] = useState("");
  const [findings, setFindings] = useState<Finding[]>([]);
  const [scanned, setScanned] = useState(false);

  async function scan() {
    const result = await invoke<Finding[]>("scan_text", { text });
    setFindings(result);
    setScanned(true);
  }

  return (
    <main style={{ maxWidth: 720, margin: "2rem auto", fontFamily: "system-ui" }}>
      <h1>Redactify</h1>
      <p>Paste text and scan it with the builtin rules — IPC plumbing proof.</p>
      <textarea
        rows={10}
        style={{ width: "100%", fontFamily: "monospace" }}
        value={text}
        onChange={(e) => setText(e.currentTarget.value)}
        placeholder="Paste log content here…"
      />
      <button onClick={scan} style={{ marginTop: "0.5rem" }}>
        Scan
      </button>
      {scanned && (
        <section>
          <h2>{findings.length} finding(s)</h2>
          <ul>
            {findings.map((f, i) => (
              <li key={i}>
                <code>{f.rule_id}</code> at {f.start}–{f.end}:{" "}
                <code>{f.matched}</code>
              </li>
            ))}
          </ul>
        </section>
      )}
    </main>
  );
}

export default App;
