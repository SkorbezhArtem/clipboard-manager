import { invoke } from '@tauri-apps/api/core';
import { getCurrentWindow } from '@tauri-apps/api/window';
import type { ClipboardItem, Settings } from './types';
import { showToast } from './toast';
import { looksLikeCode, highlightCode } from './highlight';
import 'highlight.js/styles/github-dark-dimmed.css';
import { initSearchHistoryDropdown, pushSearchHistory, hideDropdown } from './searchHistory';

function applyTheme(theme: string) {
  if (theme === 'light') {
    document.documentElement.style.setProperty('--bg-primary', '#f5f5f5');
    document.documentElement.style.setProperty('--bg-secondary', '#ffffff');
    document.documentElement.style.setProperty('--bg-hover', '#e8e8e8');
    document.documentElement.style.setProperty('--text-primary', '#1e1e2e');
    document.documentElement.style.setProperty('--text-secondary', '#6c6c6c');
    document.documentElement.style.setProperty('--border', '#d0d0d0');
  } else {
    document.documentElement.style.setProperty('--bg-primary', '#1e1e2e');
    document.documentElement.style.setProperty('--bg-secondary', '#252535');
    document.documentElement.style.setProperty('--bg-hover', '#2d2d45');
    document.documentElement.style.setProperty('--text-primary', '#cdd6f4');
    document.documentElement.style.setProperty('--text-secondary', '#9399b2');
    document.documentElement.style.setProperty('--border', '#313244');
  }
}

// State
let items: ClipboardItem[] = [];
let filteredItems: ClipboardItem[] = [];
let selectedIndex = -1;
let activeFilter = 'all';

// DOM elements
const searchInput = document.getElementById('search') as HTMLInputElement;
const pinnedSection = document.getElementById('pinned-section') as HTMLElement;
const pinnedList = document.getElementById('pinned-list') as HTMLElement;
const historyList = document.getElementById('history-list') as HTMLElement;
const closeBtn = document.getElementById('close-btn') as HTMLButtonElement;

// Initialize
async function init() {
  try {
    const s = await invoke<Settings>('get_settings');
    applyTheme(s.theme);
  } catch {}

  await loadHistory();
  setupEventListeners();
  setupKeyboardNavigation();

  initSearchHistoryDropdown(searchInput, (query) => {
    searchInput.value = query;
    searchItems(query);
  });
  
  // Reload when window gets focus (user opens app via hotkey/tray)
  getCurrentWindow().listen('tauri://focus', () => {
    loadHistory();
    searchInput.focus();
  });

  // Poll every 1 second for new clipboard items
  setInterval(() => {
    loadHistory();
  }, 1000);

  // Focus search on load
  searchInput.focus();
}

// Load clipboard history
async function loadHistory(force = false) {
  try {
    const newItems: ClipboardItem[] = await invoke('get_history', { limit: 100 });
    if (force || itemsChanged(newItems)) {
      items = newItems;
      renderItems();
    }
  } catch (error) {
    console.error('Failed to load history:', error);
  }
}

function itemsChanged(newItems: ClipboardItem[]): boolean {
  if (newItems.length !== items.length) return true;
  if (newItems.length === 0) return false;
  return newItems[0].id !== items[0].id ||
    newItems[0].created_at !== items[0].created_at ||
    newItems[0].is_pinned !== items[0].is_pinned ||
    newItems[0].tags !== items[0].tags;
}

// Search items
async function searchItems(query: string) {
  if (!query.trim()) {
    await loadHistory();
    return;
  }
  try {
    filteredItems = await invoke('search_items', { 
      query: query.trim(),
      limit: 50 
    });
    renderFilteredItems();
    pushSearchHistory(query);
  } catch (error) {
    console.error('Search failed:', error);
  }
}

// Returns the flat ordered list of visible .clipboard-item elements
function getVisibleItems(): HTMLElement[] {
  return Array.from(document.querySelectorAll('.clipboard-item')) as HTMLElement[];
}

// Flash an item briefly to give copy feedback
function flashItem(el: HTMLElement) {
  el.classList.add('item-flash');
  el.addEventListener('animationend', () => el.classList.remove('item-flash'), { once: true });
}

function isInputFocused(): boolean {
  const tag = document.activeElement?.tagName;
  return tag === 'INPUT' || tag === 'TEXTAREA';
}

// Render items
function renderItems() {
  let visible = items;

  if (activeFilter === 'pinned') {
    visible = items.filter(i => i.is_pinned);
    pinnedSection.classList.add('hidden');
    historyList.innerHTML = visible.length
      ? visible.map((item, idx) => createItemHTML(item, idx)).join('')
      : '<div class="empty-state">No pinned items</div>';
    selectedIndex = -1;
    updateSelection();
    return;
  }

  if (activeFilter === 'template') {
    visible = items.filter(i => i.is_template);
    pinnedSection.classList.add('hidden');
    historyList.innerHTML = visible.length
      ? visible.map((item, idx) => createItemHTML(item, idx)).join('')
      : '<div class="empty-state">No templates yet — star an item to save it</div>';
    selectedIndex = -1;
    updateSelection();
    return;
  }

  if (activeFilter !== 'all') {
    visible = items.filter(i => i.content_type === activeFilter);
    pinnedSection.classList.add('hidden');
    historyList.innerHTML = visible.length
      ? visible.map((item, idx) => createItemHTML(item, idx)).join('')
      : `<div class="empty-state">No ${activeFilter} items</div>`;
    selectedIndex = -1;
    updateSelection();
    return;
  }

  const pinned = visible.filter(i => i.is_pinned);
  const history = visible.filter(i => !i.is_pinned);

  if (pinned.length > 0) {
    pinnedSection.classList.remove('hidden');
    pinnedList.innerHTML = pinned.map((item, idx) => createItemHTML(item, idx)).join('');
  } else {
    pinnedSection.classList.add('hidden');
  }

  if (history.length > 0) {
    // Offset index by pinned count so 1-9 numbers are globally correct
    historyList.innerHTML = history.map((item, idx) => createItemHTML(item, idx + pinned.length)).join('');
  } else if (pinned.length === 0) {
    historyList.innerHTML = '<div class="empty-state">Clipboard is empty. Copy something!</div>';
  } else {
    historyList.innerHTML = '';
  }

  selectedIndex = -1;
  updateSelection();
}

// Render filtered search results
function renderFilteredItems() {
  pinnedSection.classList.add('hidden');
  historyList.innerHTML = filteredItems.length
    ? filteredItems.map((item, idx) => createItemHTML(item, idx)).join('')
    : '<div class="empty-state">No results found</div>';
  selectedIndex = 0;
  updateSelection();
}

// Create item HTML
function createItemHTML(item: ClipboardItem, visibleIndex: number): string {
  const icon = getIconForType(item.content_type);
  const time = formatTime(item.created_at);
  const badge = visibleIndex < 9 ? `<span class="item-badge">${visibleIndex + 1}</span>` : '';
  const classes = ['clipboard-item', item.is_pinned ? 'pinned' : '', item.is_template ? 'is-template' : ''].filter(Boolean).join(' ');

  let previewContent: string;
  if (item.content_type === 'image') {
    previewContent = `<div class="image-preview-container">
      <img src="data:image/png;base64,${item.content_preview}" class="image-preview" alt="Clipboard image" />
    </div>`;
  } else if (looksLikeCode(item.content_preview) && !searchInput.value) {
    previewContent = highlightCode(item.content_preview);
  } else {
    const highlighted = highlightSearch(item.content_preview, searchInput.value);
    previewContent = `<div class="item-preview">${highlighted}</div>`;
  }

  const tagsHtml = item.tags
    ? `<div class="item-tags">${item.tags.split(',').filter(Boolean).map(t => `<span class="tag">${t.trim()}</span>`).join('')}</div>`
    : '';

  return `
    <div class="${classes}" data-id="${item.id}" data-pinned="${item.is_pinned}" data-template="${item.is_template}">
      <div class="item-icon">${icon}${badge}</div>
      <div class="item-content">
        ${previewContent}
        ${tagsHtml}
        <div class="item-meta">
          <span>${time}</span>
          ${item.use_count > 0 ? `<span>Used ${item.use_count}×</span>` : ''}
        </div>
      </div>
      <div class="item-actions">
        <button class="template-btn" title="${item.is_template ? 'Remove template' : 'Save as template'}" aria-pressed="${item.is_template}">⭐</button>
        <button class="tag-btn" title="Edit tags">🏷️</button>
        <button class="pin-btn" title="${item.is_pinned ? 'Unpin' : 'Pin'}">${item.is_pinned ? '📌' : '📍'}</button>
        <button class="delete-btn" title="Delete">🗑️</button>
      </div>
    </div>
  `;
}

// Get icon for content type
function getIconForType(type: string): string {
  const icons: { [key: string]: string } = {
    text: '📝',
    image: '🖼️',
    file: '📎',
    html: '🌐',
  };
  return icons[type] || '📋';
}

// Format timestamp
function formatTime(timestamp: number): string {
  const date = new Date(timestamp * 1000);
  const now = new Date();
  const diff = now.getTime() - date.getTime();
  
  const minutes = Math.floor(diff / 60000);
  const hours = Math.floor(diff / 3600000);
  const days = Math.floor(diff / 86400000);
  
  if (minutes < 1) return 'Just now';
  if (minutes < 60) return `${minutes} min ago`;
  if (hours < 24) return `${hours} hour${hours > 1 ? 's' : ''} ago`;
  if (days < 7) return `${days} day${days > 1 ? 's' : ''} ago`;
  
  return date.toLocaleDateString();
}

// Escape HTML
function escapeHtml(text: string): string {
  const div = document.createElement('div');
  div.textContent = text;
  return div.innerHTML;
}

// Highlight search query in text
function highlightSearch(text: string, query: string): string {
  if (!query || !query.trim()) {
    return escapeHtml(text);
  }
  
  const escapedText = escapeHtml(text);
  const escapedQuery = query.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
  const regex = new RegExp(`(${escapedQuery})`, 'gi');
  
  return escapedText.replace(regex, '<mark>$1</mark>');
}

// Setup event listeners
function setupEventListeners() {
  // Filter tabs
  document.querySelectorAll('.filter-tab').forEach(btn => {
    btn.addEventListener('click', () => {
      document.querySelectorAll('.filter-tab').forEach(b => b.classList.remove('active'));
      btn.classList.add('active');
      activeFilter = (btn as HTMLElement).dataset.filter!;
      searchInput.value = '';
      renderItems();
    });
  });

  // Search input
  searchInput.addEventListener('input', (e) => {
    searchItems((e.target as HTMLInputElement).value);
  });
  
  // Settings button
  const settingsBtn = document.getElementById('settings-btn');
  if (settingsBtn) {
    settingsBtn.addEventListener('click', () => {
      window.location.href = 'settings.html';
    });
  }
  
  // Close button
  closeBtn.addEventListener('click', async () => {
    await invoke('hide_window');
  });
  
  // Item click handlers (event delegation)
  document.addEventListener('click', async (e) => {
    const target = e.target as HTMLElement;
    const itemEl = target.closest('.clipboard-item') as HTMLElement | null;
    if (!itemEl) return;

    const id = parseInt(itemEl.dataset.id!);

    if (target.closest('.template-btn')) {
      e.stopPropagation();
      await toggleTemplate(id, itemEl.dataset.template === 'true');
      return;
    }
    if (target.closest('.tag-btn')) {
      e.stopPropagation();
      await editTags(id);
      return;
    }
    if (target.closest('.pin-btn')) {
      e.stopPropagation();
      await togglePin(id, itemEl.dataset.pinned === 'true');
      return;
    }
    if (target.closest('.delete-btn')) {
      e.stopPropagation();
      await deleteItem(id);
      return;
    }
    // Copy item to clipboard on item click
    hideDropdown();
    flashItem(itemEl);
    await copyItem(id);
  });
}

// Keyboard navigation
function setupKeyboardNavigation() {
  document.addEventListener('keydown', async (e) => {
    const visible = getVisibleItems();

    // 1-9: quick copy by position
    if (/^[1-9]$/.test(e.key) && !isInputFocused() && !e.ctrlKey && !e.metaKey) {
      const idx = parseInt(e.key) - 1;
      if (visible[idx]) {
        e.preventDefault();
        flashItem(visible[idx]);
        await copyItem(parseInt(visible[idx].dataset.id!));
      }
      return;
    }

    switch (e.key) {
      case 'Escape':
        e.preventDefault();
        await invoke('hide_window');
        break;

      case 'ArrowDown':
        e.preventDefault();
        selectedIndex = Math.min(selectedIndex + 1, visible.length - 1);
        updateSelection();
        scrollToSelected();
        break;

      case 'ArrowUp':
        e.preventDefault();
        selectedIndex = Math.max(selectedIndex - 1, 0);
        updateSelection();
        scrollToSelected();
        break;

      case 'Enter':
        e.preventDefault();
        if (selectedIndex >= 0 && visible[selectedIndex]) {
          hideDropdown();
          flashItem(visible[selectedIndex]);
          await copyItem(parseInt(visible[selectedIndex].dataset.id!));
        }
        break;

      case 't':
      case 'T':
        if (!isInputFocused()) {
          e.preventDefault();
          if (selectedIndex >= 0 && visible[selectedIndex]) {
            const el = visible[selectedIndex];
            await toggleTemplate(parseInt(el.dataset.id!), el.dataset.template === 'true');
          }
        }
        break;

      case 'p':
      case 'P':
        if (!isInputFocused()) {
          e.preventDefault();
          if (selectedIndex >= 0 && visible[selectedIndex]) {
            const el = visible[selectedIndex];
            await togglePin(parseInt(el.dataset.id!), el.dataset.pinned === 'true');
          }
        }
        break;

      case 'd':
      case 'D':
        if (!isInputFocused()) {
          e.preventDefault();
          if (selectedIndex >= 0 && visible[selectedIndex]) {
            await deleteItem(parseInt(visible[selectedIndex].dataset.id!));
          }
        }
        break;
    }
  });
}

// Update visual selection
function updateSelection() {
  document.querySelectorAll('.clipboard-item').forEach((item, index) => {
    item.classList.toggle('selected', index === selectedIndex);
  });
}

// Scroll selected item into view
function scrollToSelected() {
  const selected = document.querySelector('.clipboard-item.selected');
  if (selected) {
    selected.scrollIntoView({ block: 'nearest', behavior: 'smooth' });
  }
}

// Copy item to clipboard
async function copyItem(id: number) {
  try {
    await invoke('copy_to_clipboard', { id });
    await invoke('hide_window');
  } catch (error) {
    console.error('Failed to copy:', error);
  }
}

// Toggle pin status
async function togglePin(id: number, isPinned: boolean) {
  try {
    await invoke('pin_item', { id, pinned: !isPinned });
    await loadHistory(true);
  } catch (error) {
    console.error('Failed to toggle pin:', error);
  }
}

// Toggle template status
async function toggleTemplate(id: number, isTemplate: boolean) {
  try {
    await invoke('mark_as_template', { id, template: !isTemplate });
    await loadHistory(true);
    showToast(isTemplate ? 'Removed from templates' : 'Saved as template ⭐', 'success');
  } catch (error) {
    console.error('Failed to toggle template:', error);
  }
}

// Edit tags via modal
function editTags(id: number): Promise<void> {
  return new Promise((resolve) => {
    const currentItem = items.find(i => i.id === id);
    const overlay = document.getElementById('tag-modal')!;
    const input = document.getElementById('tag-modal-input') as HTMLInputElement;
    const okBtn = document.getElementById('tag-modal-ok')!;
    const cancelBtn = document.getElementById('tag-modal-cancel')!;

    input.value = currentItem?.tags ?? '';
    overlay.classList.add('visible');
    setTimeout(() => input.focus(), 50);

    const finish = async (save: boolean) => {
      overlay.classList.remove('visible');
      okBtn.removeEventListener('click', onOk);
      cancelBtn.removeEventListener('click', onCancel);
      input.removeEventListener('keydown', onKey);
      if (save) {
        try {
          await invoke('update_item_tags', { id, tags: input.value });
          await loadHistory(true);
        } catch (err) {
          console.error('Failed to update tags:', err);
        }
      }
      resolve();
    };

    const onOk = () => finish(true);
    const onCancel = () => finish(false);
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Enter') finish(true);
      if (e.key === 'Escape') finish(false);
    };

    okBtn.addEventListener('click', onOk);
    cancelBtn.addEventListener('click', onCancel);
    input.addEventListener('keydown', onKey);
  });
}

// Delete item
async function deleteItem(id: number) {
  try {
    await invoke('delete_item', { id });
    await loadHistory(true);
  } catch (error) {
    console.error('Failed to delete:', error);
  }
}

// Start the app
init();
