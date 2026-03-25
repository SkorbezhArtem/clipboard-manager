import { invoke } from '@tauri-apps/api/core';

async function init() {
  const hasSetup = await invoke('has_encryption_setup');
  
  if (hasSetup) {
    showUnlockMode();
  } else {
    showSetupMode();
  }
  
  setupEventListeners();
}

function showUnlockMode() {
  (document.getElementById('header-text') as HTMLElement).textContent = 'Enter your master password to unlock';
  document.getElementById('unlock-mode')?.classList.remove('hidden');
  document.getElementById('setup-mode')?.classList.remove('active');
  (document.getElementById('password') as HTMLInputElement)?.focus();
}

function showSetupMode() {
  (document.getElementById('header-text') as HTMLElement).textContent = 'Setup encryption for your clipboard data';
  document.getElementById('unlock-mode')?.classList.add('hidden');
  document.getElementById('setup-mode')?.classList.add('active');
  (document.getElementById('new-password') as HTMLInputElement)?.focus();
}

function setupEventListeners() {
  // Unlock button
  document.getElementById('unlock-btn')?.addEventListener('click', async () => {
    await unlockEncryption();
  });
  
  // Setup button
  document.getElementById('setup-btn')?.addEventListener('click', async () => {
    await setupEncryption();
  });
  
  // Skip buttons
  document.getElementById('skip-btn')?.addEventListener('click', () => {
    window.location.href = 'index.html';
  });
  
  document.getElementById('skip-setup-btn')?.addEventListener('click', () => {
    window.location.href = 'index.html';
  });
  
  // Enter key handlers
  document.getElementById('password')?.addEventListener('keypress', (e) => {
    if (e.key === 'Enter') {
      unlockEncryption();
    }
  });
  
  document.getElementById('confirm-password')?.addEventListener('keypress', (e) => {
    if (e.key === 'Enter') {
      setupEncryption();
    }
  });
}

async function unlockEncryption() {
  const password = (document.getElementById('password') as HTMLInputElement).value;
  
  if (!password) {
    showError('Please enter your password');
    return;
  }
  
  try {
    const isValid = await invoke('verify_master_password', { password });
    
    if (!isValid) {
      showError('Invalid password');
      (document.getElementById('password') as HTMLInputElement).value = '';
      (document.getElementById('password') as HTMLInputElement).focus();
      return;
    }
    
    await invoke('unlock_encryption', { password });
    window.location.href = 'index.html';
  } catch (error) {
    showError('Failed to unlock: ' + error);
  }
}

async function setupEncryption() {
  const password = (document.getElementById('new-password') as HTMLInputElement).value;
  const confirm = (document.getElementById('confirm-password') as HTMLInputElement).value;
  
  if (!password || !confirm) {
    showError('Please fill in all fields');
    return;
  }
  
  if (password.length < 8) {
    showError('Password must be at least 8 characters');
    return;
  }
  
  if (password !== confirm) {
    showError('Passwords do not match');
    return;
  }
  
  try {
    await invoke('setup_encryption', { password });
    window.location.href = 'index.html';
  } catch (error) {
    showError('Failed to setup encryption: ' + error);
  }
}

function showError(message: string) {
  const errorEl = document.getElementById('error-message');
  if (!errorEl) return;
  errorEl.textContent = message;
  errorEl.classList.add('show');
  
  setTimeout(() => {
    errorEl.classList.remove('show');
  }, 5000);
}

init();
