import { useState, useEffect } from "react";
import { toast } from "sonner";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogFooter,
} from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import {
  importCredentials,
  ImportCredentialItem,
  importLocalCredential,
} from "@/api/credentials";
import { useQueryClient } from "@tanstack/react-query";
import { RefreshCw, Save } from "lucide-react";

interface GroupInfo {
  id: string;
  name: string;
}

interface AddCredentialDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  onImportStart?: () => void;
  onImportProgress?: (current: number, total: number) => void;
  onImportEnd?: () => void;
  /** 当前选中的分组 ID，用于新增凭证时的默认分组 */
  selectedGroupId?: string;
  /** 可用的分组列表 */
  groups?: GroupInfo[];
}

export function AddCredentialDialog({
  open,
  onOpenChange,
  onImportStart,
  onImportProgress,
  onImportEnd,
  selectedGroupId,
  groups = [],
}: AddCredentialDialogProps) {
  const [batchInput, setBatchInput] = useState("");
  const [savingLocal, setSavingLocal] = useState(false);
  // 本地选择的分组（默认使用传入的 selectedGroupId）
  const [localGroupId, setLocalGroupId] = useState(
    selectedGroupId || "default"
  );
  const queryClient = useQueryClient();

  // 当对话框打开时，同步 selectedGroupId 到 localGroupId
  useEffect(() => {
    if (open && selectedGroupId && selectedGroupId !== "all") {
      setLocalGroupId(selectedGroupId);
    }
  }, [open, selectedGroupId]);

  const resetForm = () => {
    setBatchInput("");
  };

  // 保存本地账号
  const handleSaveLocal = async () => {
    setSavingLocal(true);
    try {
      const result = await importLocalCredential();
      toast.success(result.message);
      queryClient.invalidateQueries({ queryKey: ["credentials"] });
      onOpenChange(false);
      resetForm();
    } catch (e: any) {
      toast.error(e.response?.data?.error?.message || "保存本地账号失败");
    } finally {
      setSavingLocal(false);
    }
  };

  // 批量新增
  const handleBatchImport = async () => {
    const input = batchInput.trim();
    if (!input) {
      toast.error("请输入凭证数据");
      return;
    }

    let items: ImportCredentialItem[] = [];

    // 使用本地选择的分组
    const targetGroupId = localGroupId === "all" ? "default" : localGroupId;

    // 尝试解析为 JSON
    try {
      const parsed = JSON.parse(input);
      const list = Array.isArray(parsed) ? parsed : [parsed];
      items = list
        .map((item: any) => ({
          refreshToken: item.refreshToken || item.refresh_token || item,
          authMethod: item.authMethod || item.auth_method || "social",
          groupId: targetGroupId,
        }))
        .filter(
          (item: ImportCredentialItem) =>
            item.refreshToken && typeof item.refreshToken === "string"
        );
    } catch {
      // 不是 JSON，按行分割处理
      const lines = input
        .split("\n")
        .map((l) => l.trim())
        .filter((l) => l);
      items = lines.map((token: string) => ({
        refreshToken: token,
        authMethod: "social",
        groupId: targetGroupId,
      }));
    }

    if (items.length === 0) {
      toast.error("没有有效的凭证数据");
      return;
    }

    // 关闭对话框，通知父组件开始导入
    onOpenChange(false);
    onImportStart?.();
    onImportProgress?.(0, items.length);

    // 分批添加，每批最多 10 个
    const batchSize = 10;
    let completed = 0;
    let successCount = 0;
    let failCount = 0;
    const failReasons: string[] = [];

    for (let i = 0; i < items.length; i += batchSize) {
      const batch = items.slice(i, i + batchSize);

      await Promise.all(
        batch.map(async (item) => {
          try {
            const result = await importCredentials([item]);
            // 使用后端返回的实际导入数量
            successCount += result.importedCount || 0;
            failCount += result.skippedCount || 0;
            // 收集失败原因
            if (result.skippedReasons?.length > 0) {
              failReasons.push(...result.skippedReasons);
            }
          } catch (e: any) {
            failCount++;
            failReasons.push(e.response?.data?.error?.message || e.message || '未知错误');
          }
          completed++;
          onImportProgress?.(completed, items.length);
        })
      );
    }

    onImportEnd?.();
    queryClient.invalidateQueries({ queryKey: ["credentials"] });
    resetForm();

    if (failCount > 0) {
      // 显示失败原因（最多显示前3个）
      const reasonsToShow = failReasons.slice(0, 3);
      const moreCount = failReasons.length - 3;
      let reasonText = reasonsToShow.join('; ');
      if (moreCount > 0) {
        reasonText += `; 还有 ${moreCount} 个类似错误`;
      }
      toast.error(`导入完成: ${successCount} 成功, ${failCount} 失败\n原因: ${reasonText}`, {
        duration: 8000,
      });
    } else {
      toast.success(`已添加 ${successCount} 个凭证`);
    }
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-lg">
        <DialogHeader className="flex-row items-center justify-between pr-8">
          <DialogTitle>添加凭证</DialogTitle>
          <Button
            variant="outline"
            size="sm"
            onClick={handleSaveLocal}
            disabled={savingLocal}
          >
            {savingLocal ? (
              <>
                <RefreshCw className="h-4 w-4 mr-1 animate-spin" />
                保存中...
              </>
            ) : (
              <>
                <Save className="h-4 w-4 mr-1" />
                保存本地账号
              </>
            )}
          </Button>
        </DialogHeader>

        <div className="space-y-4 py-2">
          {/* 分组选择 */}
          <div className="space-y-2">
            <label className="text-sm font-medium">添加到分组</label>
            <select
              className="w-full px-3 py-2 bg-muted border border-border rounded-md text-sm focus:outline-none focus:ring-2 focus:ring-primary"
              value={localGroupId}
              onChange={(e) => setLocalGroupId(e.target.value)}
            >
              {groups.length === 0 ? (
                <option value="default">默认分组</option>
              ) : (
                groups.map((group) => (
                  <option key={group.id} value={group.id}>
                    {group.name}
                  </option>
                ))
              )}
            </select>
          </div>

          {/* 凭证数据 */}
          <div className="space-y-2">
            <label className="text-sm font-medium">
              凭证数据 <span className="text-red-500">*</span>
            </label>
            <textarea
              className="w-full h-40 px-3 py-2 bg-muted border border-border rounded-md text-sm font-mono focus:outline-none focus:ring-2 focus:ring-primary resize-none"
              placeholder={`一行一个或JSON数组  示例：
token1
token2
token3

或 [{"refreshToken": "xxx"}]`}
              value={batchInput}
              onChange={(e) => setBatchInput(e.target.value)}
            />
            {/* 实时解析预览 */}
            {batchInput.trim() && (
              <div className="text-sm">
                {(() => {
                  const input = batchInput.trim();
                  let count = 0;
                  let isJson = false;
                  let error = "";

                  try {
                    const parsed = JSON.parse(input);
                    const list = Array.isArray(parsed) ? parsed : [parsed];
                    count = list.filter((item: any) => {
                      const token =
                        item.refreshToken ||
                        item.refresh_token ||
                        (typeof item === "string" ? item : null);
                      return (
                        token && typeof token === "string" && token.length > 10
                      );
                    }).length;
                    isJson = true;
                  } catch {
                    // 按行分割
                    const lines = input
                      .split("\n")
                      .map((l) => l.trim())
                      .filter((l) => l && l.length > 10);
                    count = lines.length;
                  }

                  if (count === 0) {
                    error = "未检测到有效凭证";
                  }

                  return (
                    <div
                      className={`flex items-center gap-2 ${
                        error ? "text-red-500" : "text-green-600"
                      }`}
                    >
                      {error ? (
                        <span>⚠️ {error}</span>
                      ) : (
                        <span>
                          ✓ 检测到 <strong>{count}</strong> 个凭证{" "}
                          {isJson ? "(JSON格式)" : "(纯文本格式)"}
                        </span>
                      )}
                    </div>
                  );
                })()}
              </div>
            )}
          </div>
        </div>

        <DialogFooter>
          <Button
            type="button"
            variant="outline"
            onClick={() => onOpenChange(false)}
          >
            取消
          </Button>
          <Button onClick={handleBatchImport}>添加</Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
