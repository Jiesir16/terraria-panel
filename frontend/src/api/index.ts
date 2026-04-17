import axios, { AxiosInstance, AxiosError } from 'axios'
import { useAuthStore } from '../stores/auth'

let instance: AxiosInstance | null = null

export function createAxiosInstance(): AxiosInstance {
  const axiosInstance = axios.create({
    baseURL: '/api',
    timeout: 30000,
    headers: {
      'Content-Type': 'application/json'
    }
  })

  // Request interceptor - add token
  axiosInstance.interceptors.request.use(
    (config) => {
      const authStore = useAuthStore()
      if (authStore.token) {
        config.headers.Authorization = `Bearer ${authStore.token}`
      }
      return config
    },
    (error) => Promise.reject(error)
  )

  // Response interceptor - handle errors
  axiosInstance.interceptors.response.use(
    (response) => response,
    (error: AxiosError) => {
      if (error.response?.status === 401) {
        const authStore = useAuthStore()
        authStore.logout()
        window.location.href = '/login'
      }
      return Promise.reject(error)
    }
  )

  return axiosInstance
}

export function getAxiosInstance(): AxiosInstance {
  if (!instance) {
    instance = createAxiosInstance()
  }
  return instance
}

export default getAxiosInstance()
