import { createContext } from './index'

export interface Notification {
  title: string
  text: string
  type: 'success' | 'error' | 'warning' | 'info'
  id?: Date
  timer?: ReturnType<typeof setTimeout>
}

export interface NotificationManagerContext {
  addNotification: (notification: Omit<Notification, 'id' | 'timer'>) => void
}

export const [injectNotificationManager, provideNotificationManager] =
  createContext<NotificationManagerContext>('NotificationManager')
