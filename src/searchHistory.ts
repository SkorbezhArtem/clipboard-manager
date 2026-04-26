const STORAGE_KEY = 'clipboard_search_history';
const MAX_ENTRIES = 10;

export function getSearchHistory(): string[] {
  try {
    return JSON.parse(localStorage.getItem(STORAGE_KEY) ?? '[]');
  } catch {
    return [];
  }
}

export function pushSearchHistory(query: string): void {
  const q = query.trim();
  if (!q || q.length < 2) return;
  const history = getSearchHistory().filter(h => h !== q);
  history.unshift(q);
  localStorage.setItem(STORAGE_KEY, JSON.stringify(history.slice(0, MAX_ENTRIES)));
}

export function clearSearchHistory(): void {
  localStorage.removeItem(STORAGE_KEY);
}

// --- Dropdown UI ---

let dropdownEl: HTMLElement | null = null;
let onSelectCallback: ((q: string) => void) | null = null;

export function initSearchHistoryDropdown(
  inputEl: HTMLInputElement,
  onSelect: (q: string) => void,
): void {
  onSelectCallback = onSelect;

  dropdownEl = document.createElement('div');
  dropdownEl.id = 'search-history-dropdown';
  dropdownEl.className = 'search-history-dropdown hidden';
  inputEl.parentElement!.style.position = 'relative';
  inputEl.parentElement!.appendChild(dropdownEl);

  inputEl.addEventListener('focus', () => {
    if (!inputEl.value) showDropdown();
  });

  inputEl.addEventListener('input', () => {
    if (!inputEl.value) showDropdown();
    else hideDropdown();
  });

  document.addEventListener('click', (e) => {
    if (!dropdownEl?.contains(e.target as Node) && e.target !== inputEl) {
      hideDropdown();
    }
  });
}

export function showDropdown(): void {
  if (!dropdownEl) return;
  const history = getSearchHistory();
  if (history.length === 0) return;

  dropdownEl.innerHTML = `
    <div class="sh-header">
      <span>Recent searches</span>
      <button class="sh-clear" title="Clear history">✕</button>
    </div>
    ${history.map(q => `<div class="sh-item" data-query="${escapeAttr(q)}">${escapeHtml(q)}</div>`).join('')}
  `;

  dropdownEl.querySelector('.sh-clear')?.addEventListener('click', (e) => {
    e.stopPropagation();
    clearSearchHistory();
    hideDropdown();
  });

  dropdownEl.querySelectorAll('.sh-item').forEach(el => {
    el.addEventListener('click', () => {
      const q = (el as HTMLElement).dataset.query!;
      hideDropdown();
      onSelectCallback?.(q);
    });
  });

  dropdownEl.classList.remove('hidden');
}

export function hideDropdown(): void {
  dropdownEl?.classList.add('hidden');
}

function escapeHtml(s: string): string {
  return s.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');
}

function escapeAttr(s: string): string {
  return s.replace(/"/g, '&quot;');
}
