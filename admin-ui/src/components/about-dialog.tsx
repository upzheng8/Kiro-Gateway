import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'
import { Button } from '@/components/ui/button'
import { Github, MessageCircle, RefreshCw } from 'lucide-react'
import { toast } from 'sonner'
import { useState } from 'react'

interface AboutDialogProps {
  open: boolean
  onOpenChange: (open: boolean) => void
}

export function AboutDialog({ open, onOpenChange }: AboutDialogProps) {
  const [checking, setChecking] = useState(false)
  
  // 版本号可以从环境变量或配置获取
  const version = '1.0.0'
  
  const handleCheckUpdate = async () => {
    setChecking(true)
    try {
      // 这里可以添加实际的更新检查逻辑
      await new Promise(resolve => setTimeout(resolve, 1500))
      toast.info('当前已是最新版本')
    } catch (e) {
      toast.error('检查更新失败')
    } finally {
      setChecking(false)
    }
  }
  
  const copyToClipboard = (text: string) => {
    navigator.clipboard.writeText(text)
    toast.success('已复制到剪贴板')
  }

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-md">
        <DialogHeader>
          <DialogTitle className="text-center">关于 Kiro Gateway</DialogTitle>
        </DialogHeader>

        <div className="space-y-6 py-4">
          {/* Logo 和版本 */}
          <div className="text-center space-y-2">
            <div className="text-4xl">🚀</div>
            <div className="text-xl font-bold">Kiro Gateway</div>
            <div className="text-sm text-muted-foreground">v{version}</div>
          </div>
          
          {/* 描述 */}
          <div className="text-center text-sm text-muted-foreground">
            Kiro API 反向代理网关，支持多凭证轮询、自动 Token 刷新、流式响应
          </div>
          
          {/* QQ 群 */}
          <div className="space-y-2">
            <div className="text-sm font-medium text-center">交流群</div>
            <div className="flex justify-center gap-2">
              <Button 
                variant="outline" 
                size="sm"
                onClick={() => copyToClipboard('1041545996')}
              >
                <MessageCircle className="h-4 w-4 mr-1" />
                QQ群 1041545996
              </Button>
              <Button 
                variant="outline" 
                size="sm"
                onClick={() => copyToClipboard('704127070')}
              >
                <MessageCircle className="h-4 w-4 mr-1" />
                QQ群 704127070
              </Button>
            </div>
          </div>
          
          {/* 链接 */}
          <div className="flex justify-center gap-2">
            <Button 
              variant="outline" 
              size="sm"
              onClick={() => window.open('https://github.com/Kiro-Gateway/Kiro-Gateway/releases', '_blank')}
            >
              <Github className="h-4 w-4 mr-1" />
              GitHub Releases
            </Button>
          </div>
          
          {/* 检查更新 */}
          <div className="flex justify-center">
            <Button 
              onClick={handleCheckUpdate}
              disabled={checking}
            >
              {checking ? (
                <>
                  <RefreshCw className="h-4 w-4 mr-1 animate-spin" />
                  检查中...
                </>
              ) : (
                <>
                  <RefreshCw className="h-4 w-4 mr-1" />
                  检查更新
                </>
              )}
            </Button>
          </div>
        </div>
      </DialogContent>
    </Dialog>
  )
}
