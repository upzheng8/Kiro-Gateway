import { useState } from 'react'
import { toast } from 'sonner'
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogFooter,
} from '@/components/ui/dialog'
import { Button } from '@/components/ui/button'
import { exportCredentials } from '@/api/credentials'
import { RefreshCw, Download } from 'lucide-react'

interface ExportDialogProps {
  open: boolean
  onOpenChange: (open: boolean) => void
  selectedIds: number[]
}

type ExportFormat = 'json' | 'tokens'

export function ExportDialog({ open, onOpenChange, selectedIds }: ExportDialogProps) {
  const [format, setFormat] = useState<ExportFormat>('tokens')
  const [exporting, setExporting] = useState(false)

  const handleExport = async () => {
    if (selectedIds.length === 0) {
      toast.error('没有选择凭证')
      return
    }

    setExporting(true)
    try {
      const result = await exportCredentials(selectedIds, format === 'json' ? 'full' : 'tokens_only')
      
      const credentials = result.credentials || []
      
      // 生成时间戳 yyyy-MM-dd HH：mm：ss（使用中文冒号避免文件名限制）
      const now = new Date()
      const pad = (n: number) => n.toString().padStart(2, '0')
      const timestamp = `${now.getFullYear()}-${pad(now.getMonth() + 1)}-${pad(now.getDate())} ${pad(now.getHours())}：${pad(now.getMinutes())}：${pad(now.getSeconds())}`
      
      let content: string
      let defaultFilename: string
      
      if (format === 'json') {
        content = JSON.stringify(credentials, null, 2)
        defaultFilename = `Kiro Gateway${credentials.length} ${timestamp}.json`
      } else {
        content = credentials.map((c: any) => c.refreshToken).join('\n')
        defaultFilename = `Kiro Gateway${credentials.length} ${timestamp}.txt`
      }
      
      // 使用 Tauri 保存对话框
      try {
        const { save } = await import('@tauri-apps/plugin-dialog')
        const { writeTextFile } = await import('@tauri-apps/plugin-fs')
        
        const filePath = await save({
          defaultPath: defaultFilename,
          filters: format === 'json' 
            ? [{ name: 'JSON', extensions: ['json'] }]
            : [{ name: 'Text', extensions: ['txt'] }]
        })
        
        if (filePath) {
          await writeTextFile(filePath, content)
          toast.success(`已导出 ${result.count} 个凭证到 ${filePath}`)
          onOpenChange(false)
        }
      } catch {
        // 回退到浏览器下载
        const mimeType = format === 'json' ? 'application/json' : 'text/plain'
        const blob = new Blob([content], { type: mimeType })
        const url = URL.createObjectURL(blob)
        const a = document.createElement('a')
        a.href = url
        a.download = defaultFilename
        a.click()
        URL.revokeObjectURL(url)
        
        toast.success(`已导出 ${result.count} 个凭证`)
        onOpenChange(false)
      }
    } catch (e: any) {
      toast.error(`导出失败: ${e.response?.data?.error?.message || e.message}`)
    } finally {
      setExporting(false)
    }
  }

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-md">
        <DialogHeader>
          <DialogTitle>导出凭证</DialogTitle>
        </DialogHeader>

        <div className="space-y-4 py-4">
          <div className="text-sm text-muted-foreground">
            已选择 {selectedIds.length} 个凭证
          </div>
          
          {/* 格式选择 */}
          <div className="space-y-3">
            <label className="text-sm font-medium">导出格式</label>
            
            <div 
              className={`p-3 border rounded-lg cursor-pointer transition-colors ${
                format === 'tokens' ? 'border-primary bg-primary/5' : 'border-border hover:border-muted-foreground'
              }`}
              onClick={() => setFormat('tokens')}
            >
              <div className="font-medium">仅 RefreshToken</div>
              <div className="text-xs text-muted-foreground mt-1">
                一行一个 Token，适合备份和批量导入
              </div>
            </div>
            
            <div 
              className={`p-3 border rounded-lg cursor-pointer transition-colors ${
                format === 'json' ? 'border-primary bg-primary/5' : 'border-border hover:border-muted-foreground'
              }`}
              onClick={() => setFormat('json')}
            >
              <div className="font-medium">JSON 格式</div>
              <div className="text-xs text-muted-foreground mt-1">
                包含完整凭证信息（Token、认证方式、优先级）
              </div>
            </div>
          </div>
        </div>

        <DialogFooter>
          <Button
            variant="outline"
            onClick={() => onOpenChange(false)}
            disabled={exporting}
          >
            取消
          </Button>
          <Button onClick={handleExport} disabled={exporting}>
            {exporting ? (
              <>
                <RefreshCw className="h-4 w-4 mr-1 animate-spin" />
                导出中...
              </>
            ) : (
              <>
                <Download className="h-4 w-4 mr-1" />
                导出
              </>
            )}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}
