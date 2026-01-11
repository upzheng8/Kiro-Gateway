// 凭证状态响应
export interface CredentialsStatusResponse {
  total: number
  available: number
  currentId: number
  credentials: CredentialStatusItem[]
}

// 单个凭证状态
export interface CredentialStatusItem {
  id: number
  priority: number
  disabled: boolean
  failureCount: number
  isCurrent: boolean
  expiresAt: string | null
  authMethod: string | null
  hasProfileArn: boolean
  email: string | null
  subscriptionTitle: string | null
  // 余额信息（缓存）
  currentUsage: number | null
  usageLimit: number | null
  remaining: number | null
  nextResetAt: number | null
  // Token 信息
  refreshToken: string | null
  accessToken: string | null
  profileArn: string | null
  // 凭证状态：normal(正常), invalid(无效/封禁), expired(过期)
  status: 'normal' | 'invalid' | 'expired'
  // 分组 ID
  groupId: string
}

// 余额响应
export interface BalanceResponse {
  id: number
  email: string | null
  subscriptionTitle: string | null
  currentUsage: number
  usageLimit: number
  remaining: number
  usagePercentage: number
  nextResetAt: number | null
  // 凭证详情
  authMethod: string | null
  accessToken: string | null
  refreshToken: string | null
  profileArn: string | null
  expiresAt: string | null
}

// 成功响应
export interface SuccessResponse {
  success: boolean
  message: string
}

// 错误响应
export interface AdminErrorResponse {
  error: {
    type: string
    message: string
  }
}

// 请求类型
export interface SetDisabledRequest {
  disabled: boolean
}

export interface SetPriorityRequest {
  priority: number
}

// 添加凭证请求
export interface AddCredentialRequest {
  refreshToken: string
  authMethod?: 'social' | 'idc' | 'builder-id'
  clientId?: string
  clientSecret?: string
  priority?: number
}

// 添加凭证响应
export interface AddCredentialResponse {
  success: boolean
  message: string
  credentialId: number
}
