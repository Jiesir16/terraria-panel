import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { authApi, UserInfo } from '../api/auth'

export const useAuthStore = defineStore('auth', () => {
  const token = ref<string | null>(null)
  const user = ref<UserInfo | null>(null)

  const isAuthenticated = computed(() => !!token.value && !!user.value)

  const isAdmin = computed(() => user.value?.role === 'admin')
  const isOperator = computed(() => user.value?.role === 'operator' || isAdmin.value)

  function initFromStorage() {
    const storedToken = localStorage.getItem('auth_token')
    if (storedToken) {
      token.value = storedToken
    }
  }

  async function login(username: string, password: string) {
    const response = await authApi.login({ username, password })
    token.value = response.data.token
    user.value = response.data.user
    localStorage.setItem('auth_token', token.value)
    return response.data
  }

  async function fetchCurrentUser() {
    try {
      const response = await authApi.getCurrentUser()
      user.value = response.data
      return response.data
    } catch (error) {
      token.value = null
      user.value = null
      localStorage.removeItem('auth_token')
      throw error
    }
  }

  async function changePassword(oldPassword: string, newPassword: string) {
    return authApi.changePassword({
      old_password: oldPassword,
      new_password: newPassword
    })
  }

  function logout() {
    token.value = null
    user.value = null
    localStorage.removeItem('auth_token')
  }

  return {
    token,
    user,
    isAuthenticated,
    isAdmin,
    isOperator,
    initFromStorage,
    login,
    fetchCurrentUser,
    changePassword,
    logout
  }
})
