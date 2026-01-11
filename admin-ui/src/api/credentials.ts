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

// 刷新单个凭证（刷新 Token + 更新余额）
export interface RefreshCredentialResponse {
  id: number;
  success: boolean;
  email: string | null;
  subscriptionTitle: string | null;
  remaining: number;
  message: string;
}

export async function refreshCredential(
  id: number
): Promise<RefreshCredentialResponse> {
  const { data } = await api.post<RefreshCredentialResponse>(
    `/credentials/${id}/refresh`
  );
  return data;
}

// 批量刷新所有凭证
export interface RefreshResultItem {
  id: number;
  success: boolean;
  email: string | null;
  remaining: number | null;
  error: string | null;
}

export interface RefreshAllResponse {
  successCount: number;
  failCount: number;
  total: number;
  results: RefreshResultItem[];
}

// 批量刷新凭证（可选传入 ID 列表）
export async function refreshAllCredentials(
  ids?: number[]
): Promise<RefreshAllResponse> {
  const { data } = await api.post<RefreshAllResponse>(
    "/credentials/refresh-all",
    { ids }
  );
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
  groupId?: string;
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
  const { data } = await api.post<ImportCredentialsResponse>(
    "/credentials/import",
    {
      credentials,
    }
  );
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

// 配置相关 API
export interface ConfigResponse {
  host: string;
  port: number;
  apiKey: string | null;
  region: string;
}

export interface UpdateConfigRequest {
  host?: string;
  port?: number;
  apiKey?: string;
  region?: string;
}

export async function getConfig(): Promise<ConfigResponse> {
  const { data } = await api.get<ConfigResponse>("/config");
  return data;
}

export async function updateConfig(
  req: UpdateConfigRequest
): Promise<SuccessResponse> {
  const { data } = await api.post<SuccessResponse>("/config", req);
  return data;
}

// ============ 批量操作 API ============

export interface BatchDeleteRequest {
  ids: number[];
}

export interface BatchDeleteResponse {
  success: boolean;
  deleted: number;
  failed: number;
  message: string;
}

export async function batchDeleteCredentials(
  ids: number[]
): Promise<BatchDeleteResponse> {
  const { data } = await api.delete<BatchDeleteResponse>("/credentials/batch", {
    data: { ids } as BatchDeleteRequest,
  });
  return data;
}

export interface ExportCredentialsRequest {
  ids: number[];
  exportType?: "full" | "tokens_only";
}

export interface ExportCredentialsResponse {
  success: boolean;
  type: string;
  count: number;
  credentials?: unknown[];
  ids?: number[];
}

export async function exportCredentials(
  ids: number[],
  exportType: "full" | "tokens_only" = "full"
): Promise<ExportCredentialsResponse> {
  const { data } = await api.post<ExportCredentialsResponse>(
    "/credentials/export",
    { ids, exportType } as ExportCredentialsRequest
  );
  return data;
}

// ============ 机器码管理 API ============

export interface MachineIdResponse {
  machineId: string | null;
  machineIdBackup: string | null;
}

export async function getMachineId(): Promise<MachineIdResponse> {
  const { data } = await api.get<MachineIdResponse>("/machine-id");
  return data;
}

export async function backupMachineId(): Promise<SuccessResponse> {
  const { data } = await api.post<SuccessResponse>("/machine-id/backup");
  return data;
}

export async function restoreMachineId(): Promise<SuccessResponse> {
  const { data } = await api.post<SuccessResponse>("/machine-id/restore");
  return data;
}

export async function resetMachineId(): Promise<SuccessResponse> {
  const { data } = await api.post<SuccessResponse>("/machine-id/reset");
  return data;
}

// ============ 模型锁定 API ============

export interface LockedModelResponse {
  lockedModel: string | null;
}

export async function getLockedModel(): Promise<LockedModelResponse> {
  const { data } = await api.get<LockedModelResponse>("/config/model");
  return data;
}

export async function setLockedModel(
  model: string | null
): Promise<SuccessResponse> {
  const { data } = await api.post<SuccessResponse>("/config/model", { model });
  return data;
}

// ============ 本地账号 API ============

export interface LocalCredentialResponse {
  success: boolean;
  hasCredential: boolean;
  refreshToken?: string;
  authMethod?: string;
  expiresAt?: string;
  error?: string;
}

export async function getLocalCredential(): Promise<LocalCredentialResponse> {
  const { data } = await api.get<LocalCredentialResponse>("/credentials/local");
  return data;
}

export async function importLocalCredential(): Promise<AddCredentialResponse> {
  const { data } = await api.post<AddCredentialResponse>(
    "/credentials/import-local"
  );
  return data;
}

export async function switchToCredential(
  id: number
): Promise<SuccessResponse> {
  const { data } = await api.post<SuccessResponse>(
    `/credentials/${id}/switch`
  );
  return data;
}

// ============ 分组管理 ============

export interface GroupInfo {
  id: string;
  name: string;
  credentialCount: number;
}

export interface GroupsResponse {
  groups: GroupInfo[];
  activeGroupId: string | null;
}

// 获取所有分组
export async function getGroups(): Promise<GroupsResponse> {
  const { data } = await api.get<GroupsResponse>("/groups");
  return data;
}

// 添加分组
export async function addGroup(name: string): Promise<SuccessResponse> {
  const { data } = await api.post<SuccessResponse>("/groups", { name });
  return data;
}

// 删除分组
export async function deleteGroup(id: string): Promise<SuccessResponse> {
  const { data } = await api.delete<SuccessResponse>(`/groups/${id}`);
  return data;
}

// 设置活跃分组
export async function setActiveGroup(groupId: string | null): Promise<SuccessResponse> {
  const { data } = await api.post<SuccessResponse>("/groups/active", { groupId });
  return data;
}

// 设置凭证分组
export async function setCredentialGroup(id: number, groupId: string): Promise<SuccessResponse> {
  const { data } = await api.post<SuccessResponse>(`/credentials/${id}/group`, { groupId });
  return data;
}

// 重命名分组
export async function renameGroup(id: string, name: string): Promise<SuccessResponse> {
  const { data } = await api.put<SuccessResponse>(`/groups/${id}`, { name });
  return data;
}

// 代理服务状态响应
export interface ProxyStatusResponse {
  running: boolean;
  host: string;
  port: number;
  activeGroupId: string | null;
}

// 获取代理服务状态
export async function getProxyStatus(): Promise<ProxyStatusResponse> {
  const { data } = await api.get<ProxyStatusResponse>("/proxy/status");
  return data;
}

// 设置代理服务启用状态
export async function setProxyEnabled(enabled: boolean): Promise<SuccessResponse> {
  const { data } = await api.post<SuccessResponse>("/proxy/enabled", { enabled });
  return data;
}
