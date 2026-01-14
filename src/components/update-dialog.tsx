import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'
import { Button } from '@/components/ui/button'
import { Download, ExternalLink } from 'lucide-react'

// 使用 Tauri invoke 打开外部链接
const openExternal = async (url: string) => {
  try {
    const { invoke } = (window as any).__TAURI__.core
    await invoke('open_url', { url })
  } catch (e) {
    console.error('打开链接失败:', e)
  }
}

// 过滤掉下载说明部分
const filterReleaseBody = (body: string): string => {
  // 移除 "### 下载说明" 及其后面的表格内容
  return body
    .replace(/###\s*下载说明[\s\S]*?(?=\n##|\n###|$)/gi, '')
    .trim()
}

interface UpdateDialogProps {
  open: boolean
  onOpenChange: (open: boolean) => void
  currentVersion: string
  latestVersion: string
  releaseUrl: string
  releaseBody: string
  publishedAt: string
}

export function UpdateDialog({
  open,
  onOpenChange,
  currentVersion,
  latestVersion,
  releaseUrl,
  releaseBody,
  publishedAt,
}: UpdateDialogProps) {
  const formattedDate = new Date(publishedAt).toLocaleDateString('zh-CN', {
    year: 'numeric',
    month: 'long',
    day: 'numeric',
  })

  // 过滤后的更新内容
  const filteredBody = filterReleaseBody(releaseBody)

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-lg">
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2"> 
            发现新版本
          </DialogTitle>
        </DialogHeader>

        <div className="space-y-4 py-4">
          {/* 版本信息 */}
          <div className="flex items-center justify-between bg-muted/50 rounded-lg p-3">
            <div className="text-sm">
              <span className="text-muted-foreground">当前版本: </span>
              <span className="font-medium">v{currentVersion}</span>
            </div>
            <div className="text-sm">
              <span className="text-muted-foreground">最新版本: </span>
              <span className="font-medium text-green-600 dark:text-green-400">v{latestVersion}</span>
            </div>
          </div>
          
          {/* 发布日期 */}
          <div className="text-xs text-muted-foreground">
            发布时间: {formattedDate}
          </div>
          
          {/* 更新内容 */}
          <div className="space-y-2">
            <div className="text-sm font-medium">更新内容:</div>
            <div className="bg-muted/30 rounded-lg p-3 max-h-60 overflow-y-auto">
              <div 
                className="text-sm text-muted-foreground prose prose-sm dark:prose-invert max-w-none"
                dangerouslySetInnerHTML={{ 
                  __html: filteredBody
                    .replace(/\n/g, '<br>')
                    .replace(/^- /gm, '• ')
                    .replace(/^## (.+)$/gm, '<strong>$1</strong>')
                    .replace(/^### (.+)$/gm, '<em>$1</em>')
                }}
              />
            </div>
          </div>
        </div>

        <div className="flex gap-3">
          <Button variant="outline" className="flex-1" onClick={() => onOpenChange(false)}>
            稍后更新
          </Button>
          <Button 
            className="flex-1" 
            onClick={() => openExternal(releaseUrl)}
          >
            <Download className="h-4 w-4 mr-2" />
            前往下载
            <ExternalLink className="h-3 w-3 ml-1 opacity-50" />
          </Button>
        </div>
      </DialogContent>
    </Dialog>
  )
}
