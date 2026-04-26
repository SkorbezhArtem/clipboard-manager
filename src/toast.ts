export type ToastType = 'success' | 'error' | 'warning' | 'info';

let container: HTMLElement | null = null;

function getContainer(): HTMLElement {
  if (!container) {
    container = document.createElement('div');
    container.id = 'toast-container';
    document.body.appendChild(container);
  }
  return container;
}

export function showToast(message: string, type: ToastType = 'info', duration = 3000) {
  const c = getContainer();

  const toast = document.createElement('div');
  toast.className = `toast toast-${type}`;

  const icons: Record<ToastType, string> = {
    success: '✓',
    error: '✕',
    warning: '⚠',
    info: 'ℹ',
  };

  toast.innerHTML = `
    <span class="toast-icon">${icons[type]}</span>
    <span class="toast-message">${message}</span>
  `;

  c.appendChild(toast);

  requestAnimationFrame(() => toast.classList.add('toast-visible'));

  setTimeout(() => {
    toast.classList.remove('toast-visible');
    toast.addEventListener('transitionend', () => toast.remove(), { once: true });
  }, duration);
}
