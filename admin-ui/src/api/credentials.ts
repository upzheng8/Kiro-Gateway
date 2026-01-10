import axios from "axios";
import type {
  CredentialsStatusResponse,
  BalanceResponse,
  SuccessResponse,
  SetDisabledRequest,
  SetPriorityRequest,
  AddCredentialRequest,
  AddCredentialResponse,
} from "@/types/api";

// 创建 axios 实例
const api = axios.create({
  baseURL:
    import.meta.env.VITE_API_BASE_URL || "http://127.0.0.1:8990/api/admin",
  headers: {
    "Content-Type": "application/json",
  },
});

// 不再需要 API Key 认证

// 获取所有凭证状态
export async function getCredentials(): Promise<CredentialsStatusResponse> {
  const { data } = await api.get<CredentialsStatusResponse>("/credentials");
  return data;
}

// 设置凭证禁用状态
export async function setCredentialDisabled(
  id: number,
  disabled: boolean
): Promise<SuccessResponse> {
  const { data } = await api.post<SuccessResponse>(
    `/credentials/${id}/disabled`,
    { disabled } as SetDisabledRequest
  );
  return data;
}

// 设置凭证优先级
export async function setCredentialPriority(
  id: number,
  priority: number
): Promise<SuccessResponse> {
  const { data } = await api.post<SuccessResponse>(
    `/credentials/${id}/priority`,
    { priority } as SetPriorityRequest
  );
  return data;
}

// 重置失败计数
export async function resetCredentialFailure(
  id: number
): Promise<SuccessResponse> {
  const { data } = await api.post<SuccessResponse>(`/credentials/${id}/reset`);
  return data;
}

// 获取凭证余额
export async function getCredentialBalance(
  id: number
): Promise<BalanceResponse> {
  const { data } = await api.get<BalanceResponse>(`/credentials/${id}/balance`);
  return data;
}

// 添加新凭证
export async function addCredential(
  req: AddCredentialRequest
): Promise<AddCredentialResponse> {
  const { data } = await api.post<AddCredentialResponse>("/credentials", req);
  return data;
}

// 删除凭证
export async function deleteCredential(id: number): Promise<SuccessResponse> {
  const { data } = await api.delete<SuccessResponse>(`/credentials/${id}`);
  return data;
}

// 批量导入凭证
export interface ImportCredentialItem {
  refreshToken: string;
  authMethod?: string;
  clientId?: string;
  clientSecret?: string;
  priority?: number;
}

export interface ImportCredentialsResponse {
  success: boolean;
  message: string;
  importedCount: number;
  skippedCount: number;
  credentialIds: number[];
}

export async function importCredentials(
  credentials: ImportCredentialItem[]
): Promise<ImportCredentialsResponse> {
  const { data } = await api.post<ImportCredentialsResponse>("/credentials/import", {
    credentials,
  });
  return data;
}

// 日志相关 API
export interface LogEntry {
  timestamp: string;
  level: string;
  message: string;
  request?: {
    model: string;
    maxTokens: number;
    stream: boolean;
    messageCount: number;
    systemPreview: string;
    userMessagePreview: string;
  };
  response?: {
    model: string;
    inputTokens: number;
    outputTokens: number;
    stopReason: string;
    hasToolUse: boolean;
    responsePreview: string;
  };
}

export interface LogsResponse {
  logs: LogEntry[];
  total: number;
}

export async function getLogs(): Promise<LogsResponse> {
  const { data } = await api.get<LogsResponse>("/logs");
  return data;
}

export async function clearLogs(): Promise<SuccessResponse> {
  const { data } = await api.post<SuccessResponse>("/logs/clear");
  return data;
}
