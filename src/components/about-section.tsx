import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Button } from '@/components/ui/button'
import { Github, MessageCircle, RefreshCw, ExternalLink, Code2, Server, BookOpen } from 'lucide-react'
import { toast } from 'sonner'
import { useState, useEffect } from 'react'
import { getVersion, checkUpdate } from '@/api/credentials'
import { UpdateDialog } from '@/components/update-dialog'

export function AboutSection() {
  const [checking, setChecking] = useState(false)
  const [version, setVersion] = useState('...')
  const [updateDialogOpen, setUpdateDialogOpen] = useState(false)
  const [updateInfo, setUpdateInfo] = useState<{
    currentVersion: string;
    latestVersion: string;
    releaseUrl: string;
    releaseBody: string;
    publishedAt: string;
  } | null>(null)
  
  // 加载版本号（不再自动检查更新，由 Dashboard 统一处理）
  useEffect(() => {
    getVersion().then(res => {
      setVersion(res.version)
    }).catch(() => {
      setVersion('未知')
    })
  }, [])
  
  // 手动检查更新
  const handleCheckUpdate = async () => {
    setChecking(true)
    try {
      const result = await checkUpdate()
      if (result.hasUpdate) {
        setUpdateInfo({
          currentVersion: result.currentVersion,
          latestVersion: result.latestVersion,
          releaseUrl: result.releaseUrl,
          releaseBody: result.releaseBody,
          publishedAt: result.publishedAt,
        })
        setUpdateDialogOpen(true)
      } else {
        toast.success('当前已是最新版本')
      }
    } catch (e: any) {
      console.error('检查更新失败:', e)
      toast.error(e.message || '检查更新失败')
    } finally {
      setChecking(false)
    }
  }

  const techStack = [
    { icon: Code2, label: '前端', value: 'React + Vite', color: 'text-cyan-500' },
    { icon: Server, label: '后端', value: 'Tauri + Rust', color: 'text-orange-500' },
  ]

  return (
    <div className="space-y-6">
      {/* 主卡片 */}
      <Card>
        <CardContent className="pt-6 space-y-4">
          <div className="flex items-center justify-center gap-6">
            {/* 左侧 Logo */}
            <img src="/icon.png" alt="Kiro Gateway" className="w-20 h-20 rounded-2xl shadow-xl flex-shrink-0" />
            
            {/* 右侧内容 */}
            <div className="flex-1 min-w-0">
              <h1 className="text-2xl font-bold">Kiro Gateway</h1>
              <div className="flex items-center gap-2 mt-1">
                <span className="px-3 py-1 bg-primary/10 text-primary rounded-full text-sm font-medium">
                  v{version}
                </span>
                <Button 
                  variant="ghost" 
                  size="sm"
                  onClick={handleCheckUpdate}
                  disabled={checking}
                  className="h-7 text-xs"
                >
                  <RefreshCw className={`h-3 w-3 mr-1 ${checking ? 'animate-spin' : ''}`} />
                  {checking ? '检查中...' : '检查更新'}
                </Button>
              </div>
            </div>
          </div>
        </CardContent>
      </Card>

      {/* 相关链接 & 交流群 */}
      <Card>
        <CardHeader className="pb-3">
          <CardTitle className="text-sm">相关链接</CardTitle>
        </CardHeader>
        <CardContent className="space-y-3">
          {/* GitHub 和 使用文档 */}
          <div className="flex gap-3">
            <button 
              onClick={async () => {
                try {
                  const { invoke } = (window as any).__TAURI__.core
                  await invoke('open_url', { url: 'https://github.com/Zheng-up/KiroGateway-release/releases' })
                } catch (e) {
                  console.error('打开链接失败:', e)
                }
              }}
              className="flex-1 flex items-center justify-center gap-2 h-10 bg-gray-900 dark:bg-gray-800 hover:bg-gray-800 dark:hover:bg-gray-700 rounded-lg transition-colors group cursor-pointer"
            >
              <Github className="h-4 w-4 text-white" />
              <span className="text-white text-xs font-medium">GitHub 发布</span>
              <ExternalLink className="h-3 w-3 text-white/50 group-hover:text-white" />
            </button>
            <button 
              onClick={async () => {
                try {
                  const { invoke } = (window as any).__TAURI__.core
                  await invoke('open_url', { url: 'https://docs.qq.com/aio/DT0ZvQm1kc2ZYZUZO' })
                } catch (e) {
                  console.error('打开链接失败:', e)
                }
              }}
              className="flex-1 flex items-center justify-center gap-2 h-10 bg-green-600 hover:bg-green-500 rounded-lg transition-colors group cursor-pointer"
            >
              <BookOpen className="h-4 w-4 text-white" />
              <span className="text-white text-xs font-medium">使用文档</span>
              <ExternalLink className="h-3 w-3 text-white/50 group-hover:text-white" />
            </button>
          </div>
          
          {/* QQ 群 */}
          <div className="flex gap-3">
            <button 
              onClick={async () => {
                try {
                  const { invoke } = (window as any).__TAURI__.core
                  await invoke('open_url', { url: 'https://qm.qq.com/q/PoZMrdXTeA' })
                } catch (e) {
                  console.error('打开链接失败:', e)
                }
              }}
              className="flex-1 flex items-center justify-center gap-2 h-10 bg-blue-500 hover:bg-blue-600 rounded-lg transition-colors group cursor-pointer"
            >
              <MessageCircle className="h-4 w-4 text-white" />
              <span className="text-white text-xs font-medium">QQ一群 1041545996</span>
              <ExternalLink className="h-3 w-3 text-white/50 group-hover:text-white" />
            </button>
            <button 
              onClick={async () => {
                try {
                  const { invoke } = (window as any).__TAURI__.core
                  await invoke('open_url', { url: 'https://qm.qq.com/q/p9Q6VT9tFm' })
                } catch (e) {
                  console.error('打开链接失败:', e)
                }
              }}
              className="flex-1 flex items-center justify-center gap-2 h-10 bg-blue-500 hover:bg-blue-600 rounded-lg transition-colors group cursor-pointer"
            >
              <MessageCircle className="h-4 w-4 text-white" />
              <span className="text-white text-xs font-medium">QQ二群 704127070</span>
              <ExternalLink className="h-3 w-3 text-white/50 group-hover:text-white" />
            </button>
          </div>
        </CardContent>
      </Card>

      {/* 技术栈 - 放在最下面 */}
      <Card>
        <CardHeader className="pb-3">
          <CardTitle className="text-sm">技术栈</CardTitle>
        </CardHeader>
        <CardContent>
          <div className="flex gap-3">
            {techStack.map(({ icon: Icon, label, value, color }) => (
              <div key={label} className="flex-1 flex items-center gap-2 bg-muted/50 rounded-lg p-2.5">
                <Icon className={`h-4 w-4 ${color}`} />
                <span className="text-xs text-muted-foreground">{label}</span>
                <span className="text-xs font-medium ml-auto">{value}</span>
              </div>
            ))}
          </div>
        </CardContent>
      </Card>
      
      {/* 更新弹窗（仅手动检查触发） */}
      {updateInfo && (
        <UpdateDialog
          open={updateDialogOpen}
          onOpenChange={setUpdateDialogOpen}
          currentVersion={updateInfo.currentVersion}
          latestVersion={updateInfo.latestVersion}
          releaseUrl={updateInfo.releaseUrl}
          releaseBody={updateInfo.releaseBody}
          publishedAt={updateInfo.publishedAt}
        />
      )}
    </div>
  )
}
