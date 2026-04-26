import hljs from 'highlight.js/lib/core';

// Register only the most common languages to keep bundle small
import javascript from 'highlight.js/lib/languages/javascript';
import typescript from 'highlight.js/lib/languages/typescript';
import python from 'highlight.js/lib/languages/python';
import rust from 'highlight.js/lib/languages/rust';
import css from 'highlight.js/lib/languages/css';
import xml from 'highlight.js/lib/languages/xml';
import json from 'highlight.js/lib/languages/json';
import bash from 'highlight.js/lib/languages/bash';
import sql from 'highlight.js/lib/languages/sql';
import cpp from 'highlight.js/lib/languages/cpp';

hljs.registerLanguage('javascript', javascript);
hljs.registerLanguage('typescript', typescript);
hljs.registerLanguage('python', python);
hljs.registerLanguage('rust', rust);
hljs.registerLanguage('css', css);
hljs.registerLanguage('xml', xml);
hljs.registerLanguage('json', json);
hljs.registerLanguage('bash', bash);
hljs.registerLanguage('sql', sql);
hljs.registerLanguage('cpp', cpp);

const CODE_PATTERNS = [
  /^\s*(import|export|from|require)\s/m,
  /^\s*(function|const|let|var|class|def|fn |pub fn|async fn)\s/m,
  /^\s*(if|for|while|return|switch|match)\s*[\w(]/m,
  /[{};]\s*\n/,
  /^\s*#(include|define|pragma|import)/m,
  /^\s*(SELECT|INSERT|UPDATE|DELETE|CREATE|DROP)\s/im,
  /^\s*<\?xml|<!DOCTYPE|<html/i,
  /^\s*\{[\s\S]*"[\w]+"\s*:/,
];

const MIN_CODE_LENGTH = 30;
const MIN_NEWLINES = 1;

/** Returns true if the text looks like a code snippet */
export function looksLikeCode(text: string): boolean {
  if (text.length < MIN_CODE_LENGTH) return false;
  if ((text.match(/\n/g) ?? []).length < MIN_NEWLINES) return false;
  return CODE_PATTERNS.some(re => re.test(text));
}

/** Highlight text as code; returns HTML string with <code> wrapper */
export function highlightCode(text: string): string {
  try {
    const result = hljs.highlightAuto(text);
    return `<pre class="code-preview"><code class="hljs language-${result.language ?? ''}">${result.value}</code></pre>`;
  } catch {
    const escaped = text.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');
    return `<pre class="code-preview"><code class="hljs">${escaped}</code></pre>`;
  }
}
