import axios from "axios";
import type {
  CredentialsStatusResponse,
  BalanceResponse,
  SuccessResponse,
  SetDisabledRequest,
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
  groupId?: string;
}

export interface ImportCredentialsResponse {
  success: boolean;
  message: string;
  importedCount: number;
  skippedCount: number;
  credentialIds: number[];
  skippedReasons: string[];
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
  proxyPort: number;
  apiKey: string | null;
  region: string;
  autoRefreshEnabled: boolean;
  autoRefreshIntervalMinutes: number;
  lockedModel: string | null;
  machineIdBackup: string | null;
}

export interface UpdateConfigRequest {
  host?: string;
  port?: number;
  proxyPort?: number;
  apiKey?: string;
  region?: string;
  autoRefreshEnabled?: boolean;
  autoRefreshIntervalMinutes?: number;
  lockedModel?: string;
  machineIdBackup?: string;
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

export interface MachineIdBackup {
  machineId: string;
  backupTime: string;
}

export interface MachineIdResponse {
  machineId: string | null;
  machineIdBackup: MachineIdBackup | null;
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

// 切换到下一个可用凭证（反代使用）
export interface SwitchNextResponse {
  success: boolean;
  message: string;
  currentId: number;
}

export async function switchToNextCredential(): Promise<SwitchNextResponse> {
  const { data } = await api.post<SwitchNextResponse>(
    `/credentials/switch-next`
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

// 版本信息响应
export interface VersionResponse {
  version: string;
  name: string;
}

// 获取当前版本信息
export async function getVersion(): Promise<VersionResponse> {
  const { data } = await api.get<VersionResponse>("/version");
  return data;
}

// GitHub Release 信息
export interface GitHubRelease {
  tag_name: string;
  name: string;
  html_url: string;
  published_at: string;
  body: string;
}

// 检查更新（从 GitHub Releases 获取最新版本）
export async function checkUpdate(): Promise<{
  hasUpdate: boolean;
  currentVersion: string;
  latestVersion: string;
  releaseUrl: string;
  releaseBody: string;
  publishedAt: string;
}> {
  // 获取当前版本
  const { version: currentVersion } = await getVersion();
  
  // 从 GitHub Releases 获取最新版本
  const response = await fetch(
    'https://api.github.com/repos/Zheng-up/KiroGateway-release/releases/latest'
  );
  
  if (!response.ok) {
    throw new Error('无法获取更新信息');
  }
  
  const release: GitHubRelease = await response.json();
  const latestVersion = release.tag_name.replace(/^v/, '');
  
  // 比较版本号
  const hasUpdate = compareVersions(latestVersion, currentVersion) > 0;
  
  return {
    hasUpdate,
    currentVersion,
    latestVersion,
    releaseUrl: release.html_url,
    releaseBody: release.body || '暂无更新说明',
    publishedAt: release.published_at,
  };
}

// 版本号比较函数
function compareVersions(v1: string, v2: string): number {
  const parts1 = v1.split('.').map(Number);
  const parts2 = v2.split('.').map(Number);
  
  for (let i = 0; i < Math.max(parts1.length, parts2.length); i++) {
    const p1 = parts1[i] || 0;
    const p2 = parts2[i] || 0;
    if (p1 > p2) return 1;
    if (p1 < p2) return -1;
  }
  return 0;
}
