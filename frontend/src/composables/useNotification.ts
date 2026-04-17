import { useAppStore } from '../stores/app'

export function useNotification() {
  const appStore = useAppStore()

  function success(title: string, message: string, duration?: number) {
    return appStore.addNotification('success', title, message, duration)
  }

  function error(title: string, message: string, duration?: number) {
    return appStore.addNotification('error', title, message, duration)
  }

  function warning(title: string, message: string, duration?: number) {
    return appStore.addNotification('warning', title, message, duration)
  }

  function info(title: string, message: string, duration?: number) {
    return appStore.addNotification('info', title, message, duration)
  }

  return {
    success,
    error,
    warning,
    info
  }
}
