import { invoke } from '@tauri-apps/api/core';
import type { Settings, Statistics } from './types';
import { showToast } from './toast';

function showPasswordModal(title: string): Promise<string | null> {
  return new Promise((resolve) => {
    const overlay = document.getElementById('pwd-modal')!;
    const titleEl = document.getElementById('pwd-modal-title')!;
    const input = document.getElementById('pwd-modal-input') as HTMLInputElement;
    const okBtn = document.getElementById('pwd-modal-ok')!;
    const cancelBtn = document.getElementById('pwd-modal-cancel')!;

    titleEl.textContent = title;
    input.value = '';
    overlay.classList.add('visible');
    setTimeout(() => input.focus(), 50);

    const finish = (value: string | null) => {
      overlay.classList.remove('visible');
      okBtn.removeEventListener('click', onOk);
      cancelBtn.removeEventListener('click', onCancel);
      input.removeEventListener('keydown', onKey);
      resolve(value);
    };

    const onOk = () => finish(input.value || null);
    const onCancel = () => finish(null);
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Enter') finish(input.value || null);
      if (e.key === 'Escape') finish(null);
    };

    okBtn.addEventListener('click', onOk);
    cancelBtn.addEventListener('click', onCancel);
    input.addEventListener('keydown', onKey);
  });
}

let currentSettings: Settings | null = null;

async function init() {
  await loadSettings();
  await loadStatistics();
  await loadEncryptionStatus();
  setupEventListeners();
}

async function loadSettings() {
  try {
    currentSettings = await invoke<Settings>('get_settings');
    
    (document.getElementById('history-limit') as HTMLInputElement).value = String(currentSettings.history_limit);
    (document.getElementById('auto-cleanup') as HTMLInputElement).checked = currentSettings.auto_cleanup_enabled;
    (document.getElementById('cleanup-days') as HTMLInputElement).value = String(currentSettings.auto_cleanup_days);
    (document.getElementById('cleanup-text-days') as HTMLInputElement).value = String(currentSettings.cleanup_text_days);
    (document.getElementById('cleanup-image-days') as HTMLInputElement).value = String(currentSettings.cleanup_image_days);
    (document.getElementById('theme') as HTMLSelectElement).value = currentSettings.theme;
    (document.getElementById('custom-hotkey') as HTMLInputElement).value = currentSettings.custom_hotkey;
    
    applyTheme(currentSettings.theme);
  } catch (error) {
    console.error('Failed to load settings:', error);
  }
}

async function loadStatistics() {
  try {
    const stats = await invoke<Statistics>('get_statistics');
    
    (document.getElementById('total-items') as HTMLElement).textContent = String(stats.total_items);
    (document.getElementById('pinned-items') as HTMLElement).textContent = String(stats.pinned_items);
    (document.getElementById('text-items') as HTMLElement).textContent = String(stats.text_items);
    (document.getElementById('image-items') as HTMLElement).textContent = String(stats.image_items);
  } catch (error) {
    console.error('Failed to load statistics:', error);
  }
}

function setupEventListeners() {
  document.getElementById('back-btn')?.addEventListener('click', () => {
    window.location.href = 'index.html';
  });
  
  document.getElementById('save-btn')?.addEventListener('click', async () => {
    await saveSettings();
  });
  
  document.getElementById('cancel-btn')?.addEventListener('click', () => {
    window.location.href = 'index.html';
  });
  
  document.getElementById('theme')?.addEventListener('change', (e) => {
    applyTheme((e.target as HTMLSelectElement).value);
  });
  
  document.getElementById('export-btn')?.addEventListener('click', async () => {
    await exportHistory();
  });
  
  document.getElementById('import-btn')?.addEventListener('click', () => {
    (document.getElementById('import-file') as HTMLInputElement)?.click();
  });
  
  document.getElementById('import-file')?.addEventListener('change', async (e) => {
    const files = (e.target as HTMLInputElement).files;
    if (files?.[0]) await importHistory(files[0]);
  });
  
  document.getElementById('setup-encryption-btn')?.addEventListener('click', async () => {
    await setupEncryption();
  });
  
  document.getElementById('change-password-btn')?.addEventListener('click', async () => {
    await changePassword();
  });
  
  document.getElementById('lock-btn')?.addEventListener('click', async () => {
    await lockEncryption();
  });
}

async function loadEncryptionStatus() {
  try {
    const isEnabled = await invoke<boolean>('is_encryption_enabled');
    const hasSetup = await invoke<boolean>('has_encryption_setup');
    
    const indicator = document.getElementById('encryption-indicator');
    const setupBtn = document.getElementById('setup-encryption-btn') as HTMLButtonElement;
    const changeBtn = document.getElementById('change-password-btn') as HTMLButtonElement;
    const lockBtn = document.getElementById('lock-btn') as HTMLButtonElement;
    
    if (!indicator) return;
    
    if (!hasSetup) {
      indicator.innerHTML = '<span style="color: var(--text-secondary);">⚪ Not configured</span>';
      setupBtn.style.display = 'block';
      changeBtn.style.display = 'none';
      lockBtn.style.display = 'none';
    } else if (isEnabled) {
      indicator.innerHTML = '<span style="color: #a6e3a1;">🟢 Unlocked</span>';
      setupBtn.style.display = 'none';
      changeBtn.style.display = 'block';
      lockBtn.style.display = 'block';
    } else {
      indicator.innerHTML = '<span style="color: var(--warning);">🔒 Locked</span>';
      setupBtn.style.display = 'none';
      changeBtn.style.display = 'block';
      lockBtn.style.display = 'none';
    }
  } catch (error) {
    console.error('Failed to load encryption status:', error);
  }
}

async function setupEncryption() {
  const newPassword = await showPasswordModal('Enter master password (min 8 chars)');
  if (!newPassword || newPassword.length < 8) {
    showToast('Password must be at least 8 characters', 'warning');
    return;
  }
  
  const confirm = await showPasswordModal('Confirm password');
  if (newPassword !== confirm) {
    showToast('Passwords do not match', 'error');
    return;
  }
  
  try {
    await invoke('setup_encryption', { password: newPassword });
    showToast('Encryption set up successfully!', 'success');
    await loadEncryptionStatus();
  } catch (error) {
    showToast('Failed to setup encryption: ' + error, 'error');
  }
}

async function changePassword() {
  const oldPassword = await showPasswordModal('Enter current master password');
  if (!oldPassword) return;
  
  const isValid = await invoke<boolean>('verify_master_password', { password: oldPassword });
  if (!isValid) {
    showToast('Invalid password', 'error');
    return;
  }
  
  const newPassword = await showPasswordModal('Enter new master password (min 8 chars)');
  if (!newPassword || newPassword.length < 8) {
    showToast('Password must be at least 8 characters', 'warning');
    return;
  }
  
  const confirm = await showPasswordModal('Confirm new password');
  if (newPassword !== confirm) {
    showToast('Passwords do not match', 'error');
    return;
  }
  
  try {
    await invoke('setup_encryption', { password: newPassword });
    showToast('Master password changed successfully!', 'success');
    await loadEncryptionStatus();
  } catch (error) {
    showToast('Failed to change password: ' + error, 'error');
  }
}

async function lockEncryption() {
  try {
    await invoke('lock_encryption');
    showToast('Encryption locked', 'info');
    await loadEncryptionStatus();
  } catch (error) {
    showToast('Failed to lock: ' + error, 'error');
  }
}

async function saveSettings() {
  try {
    const settings = {
      history_limit: parseInt((document.getElementById('history-limit') as HTMLInputElement).value),
      auto_cleanup_enabled: (document.getElementById('auto-cleanup') as HTMLInputElement).checked,
      auto_cleanup_days: parseInt((document.getElementById('cleanup-days') as HTMLInputElement).value),
      cleanup_text_days: parseInt((document.getElementById('cleanup-text-days') as HTMLInputElement).value) || 0,
      cleanup_image_days: parseInt((document.getElementById('cleanup-image-days') as HTMLInputElement).value) || 0,
      theme: (document.getElementById('theme') as HTMLSelectElement).value,
      custom_hotkey: (document.getElementById('custom-hotkey') as HTMLInputElement).value,
    };
    
    await invoke('update_settings', { settings });
    showToast('Settings saved! Restart to apply hotkey changes.', 'success');
  } catch (error) {
    console.error('Failed to save settings:', error);
    showToast('Failed to save settings: ' + error, 'error');
  }
}

async function exportHistory() {
  try {
    const filePath = await invoke<string>('export_history_file');
    showToast(`Exported to Downloads folder`, 'success', 4000);
    console.log('Export path:', filePath);
  } catch (error) {
    console.error('Failed to export history:', error);
    showToast('Failed to export: ' + error, 'error');
  }
}

async function importHistory(file: File) {
  if (!file) return;
  
  try {
    const text = await file.text();
    await invoke('import_history', { data: text });
    showToast('History imported successfully!', 'success');
    await loadStatistics();
  } catch (error) {
    console.error('Failed to import history:', error);
    showToast('Failed to import: ' + error, 'error');
  }
}

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

init();
