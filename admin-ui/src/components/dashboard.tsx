import { useState, useEffect, useRef } from 'react'
import { RefreshCw, Moon, Sun, Server, Plus, Settings, FolderOpen, Terminal, Save, Trash2, ToggleLeft, ToggleRight, DollarSign, RotateCcw, ChevronUp, ChevronDown } from 'lucide-react'
import { useQueryClient } from '@tanstack/react-query'
import { toast } from 'sonner'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Button } from '@/components/ui/button'
import { Badge } from '@/components/ui/badge'
import { BalanceDialog } from '@/components/balance-dialog'
import { AddCredentialDialog } from '@/components/add-credential-dialog'
import { useCredentials } from '@/hooks/use-credentials'
import { setCredentialDisabled, setCredentialPriority, resetCredentialFailure, deleteCredential, importCredentials, ImportCredentialItem, getCredentialBalance } from '@/api/credentials'

interface DashboardProps {
  onLogout?: () => void
}

// 输入框组件
function FormInput({
  label,
  value,
  onChange,
  type = 'text',
  placeholder,
  disabled = false,
}: {
  label: string
  value: string
  onChange: (value: string) => void
  type?: string
  placeholder?: string
  disabled?: boolean
}) {
  return (
    <div className="space-y-1.5">
      <label className="text-xs font-medium text-muted-foreground">{label}</label>
      <input
        type={type}
        value={value}
        onChange={(e) => onChange(e.target.value)}
        placeholder={placeholder}
        disabled={disabled}
        className={`w-full px-3 py-2 bg-muted border border-border rounded-md text-sm focus:outline-none focus:ring-2 focus:ring-primary ${disabled ? 'opacity-50 cursor-not-allowed' : ''}`}
      />
    </div>
  )
}

// 侧边栏导航项
function NavItem({
  icon,
  label,
  active,
  onClick,
}: {
  icon: React.ReactNode
  label: string
  active: boolean
  onClick: () => void
}) {
  return (
    <button
      onClick={onClick}
      className={`w-full flex items-center gap-3 px-3 py-2.5 rounded-lg text-sm font-medium transition-colors
        ${active 
          ? 'bg-primary text-primary-foreground' 
          : 'text-muted-foreground hover:text-foreground hover:bg-muted'
        }`}
    >
      {icon}
      {label}
    </button>
  )
}

export function Dashboard(_props: DashboardProps) {
  const [selectedCredentialId, setSelectedCredentialId] = useState<number | null>(null)
  const [balanceDialogOpen, setBalanceDialogOpen] = useState(false)
  const [addDialogOpen, setAddDialogOpen] = useState(false)
  const [activeTab, setActiveTab] = useState('credentials')
  const [darkMode, setDarkMode] = useState(() => {
    if (typeof window !== 'undefined') {
      return document.documentElement.classList.contains('dark')
    }
    return false
  })

  // 配置状态
  const [configHost, setConfigHost] = useState('')
  const [configPort, setConfigPort] = useState('')
  const [configApiKey, setConfigApiKey] = useState('')
  const [configLoading, setConfigLoading] = useState(true)
  const [configSaving, setConfigSaving] = useState(false)
  
  // 导入状态
  const [importing, setImporting] = useState(false)
  
  // 日志状态 - 使用 LogEntry 类型
  const [logs, setLogs] = useState<import('@/api/credentials').LogEntry[]>([])
  const [localLogs, setLocalLogs] = useState<string[]>(['[System] Kiro Gateway 已启动'])
  const logsEndRef = useRef<HTMLDivElement>(null)
  const logIntervalRef = useRef<ReturnType<typeof setInterval> | null>(null)
  const fileInputRef = useRef<HTMLInputElement>(null)

  useQueryClient() // keep hook call for potential future use
  const { data, isLoading, error, refetch } = useCredentials()
  
  // 凭证余额缓存
  const [balances, setBalances] = useState<Record<number, { remaining: number; loading: boolean }>>({});

  // 从后端获取真实日志
  useEffect(() => {
    const fetchLogs = async () => {
      try {
        const { getLogs } = await import('@/api/credentials')
        const response = await getLogs()
        setLogs(response.logs)
      } catch (e) {
        // 忽略错误，保持当前日志
      }
    }
    
    // 立即获取一次
    fetchLogs()
    
    // 每 2 秒获取一次
    logIntervalRef.current = setInterval(fetchLogs, 2000)
    
    return () => {
      if (logIntervalRef.current) {
        clearInterval(logIntervalRef.current)
      }
    }
  }, [])

  // 启动时从后端加载配置
  useEffect(() => {
    const loadConfig = async () => {
      try {
        const { getConfig } = await import('@/api/credentials')
        const config = await getConfig()
        setConfigHost(config.host)
        setConfigPort(config.port.toString())
        setConfigApiKey(config.apiKey || '')
      } catch (e) {
        // 忽略错误，使用默认值
        setConfigHost('127.0.0.1')
        setConfigPort('8990')
        setConfigApiKey('')
      } finally {
        setConfigLoading(false)
      }
    }
    loadConfig()
  }, [])

  // 加载凭证列表后获取余额
  useEffect(() => {
    if (!data?.credentials) return;
    
    // 为每个凭证获取余额
    data.credentials.forEach(async (cred) => {
      // 跳过已禁用的凭证
      if (cred.disabled) {
        setBalances(prev => ({ ...prev, [cred.id]: { remaining: -1, loading: false } }));
        return;
      }
      
      // 跳过已缓存的
      if (balances[cred.id] !== undefined) return;
      
      // 标记为加载中
      setBalances(prev => ({ ...prev, [cred.id]: { remaining: 0, loading: true } }));
      
      try {
        const balance = await getCredentialBalance(cred.id);
        setBalances(prev => ({ ...prev, [cred.id]: { remaining: balance.remaining, loading: false } }));
      } catch (e) {
        setBalances(prev => ({ ...prev, [cred.id]: { remaining: -1, loading: false } }));
      }
    });
  }, [data?.credentials]);

  useEffect(() => {
    logsEndRef.current?.scrollIntoView({ behavior: 'smooth' })
  }, [logs])

  const toggleDarkMode = () => {
    setDarkMode(!darkMode)
    document.documentElement.classList.toggle('dark')
  }

  // 刷新单个凭证的余额
  const refreshBalance = async (id: number) => {
    setBalances(prev => ({ ...prev, [id]: { remaining: 0, loading: true } }));
    try {
      const balance = await getCredentialBalance(id);
      setBalances(prev => ({ ...prev, [id]: { remaining: balance.remaining, loading: false } }));
    } catch (e) {
      setBalances(prev => ({ ...prev, [id]: { remaining: -1, loading: false } }));
    }
  }

  const handleViewBalance = async (id: number) => {
    // 点击查看余额时强制刷新
    await refreshBalance(id);
    setSelectedCredentialId(id)
    setBalanceDialogOpen(true)
  }

  const handleRefresh = () => {
    refetch()
    toast.success('已刷新凭证列表')
    addLog('[System] 已刷新凭证列表')
  }

  const addLog = (message: string) => {
    const timestamp = new Date().toLocaleTimeString()
    setLocalLogs(prev => [...prev.slice(-199), `[${timestamp}] ${message}`])
  }

  const handleSaveConfig = async () => {
    const port = parseInt(configPort, 10)
    if (isNaN(port) || port < 1 || port > 65535) {
      toast.error('端口号必须是 1-65535 之间的数字')
      return
    }
    
    setConfigSaving(true)
    try {
      const { updateConfig } = await import('@/api/credentials')
      const result = await updateConfig({
        host: configHost,
        port: port,
        apiKey: configApiKey || undefined,
      })
      toast.success(result.message)
      addLog('[System] 配置已保存')
    } catch (e) {
      toast.error('保存配置失败')
      addLog('[Error] 保存配置失败')
    } finally {
      setConfigSaving(false)
    }
  }

  const handleImportFile = async (event: React.ChangeEvent<HTMLInputElement>) => {
    const file = event.target.files?.[0]
    if (!file) return

    setImporting(true)
    try {
      const text = await file.text()
      const jsonData = JSON.parse(text)
      
      // 支持两种格式：数组或单个对象
      const credentialsList = Array.isArray(jsonData) ? jsonData : [jsonData]
      
      // 转换为 API 格式
      const items: ImportCredentialItem[] = credentialsList.map((cred: any) => ({
        refreshToken: cred.refreshToken || cred.refresh_token,
        authMethod: cred.authMethod || cred.auth_method || 'social',
        clientId: cred.clientId || cred.client_id,
        clientSecret: cred.clientSecret || cred.client_secret,
        priority: cred.priority || 0,
      })).filter((item: ImportCredentialItem) => item.refreshToken) // 过滤掉没有 refreshToken 的

      if (items.length === 0) {
        toast.error('文件中没有有效的凭证数据')
        return
      }

      addLog(`[System] 开始导入 ${items.length} 个凭证...`)
      const result = await importCredentials(items)
      
      toast.success(result.message)
      addLog(`[System] ${result.message}`)
      refetch()
    } catch (e) {
      const error = e as Error
      if (error.message.includes('JSON')) {
        toast.error('JSON 格式错误，请检查文件内容')
      } else {
        toast.error(`导入失败: ${error.message}`)
      }
      addLog(`[Error] 导入失败: ${error.message}`)
    } finally {
      setImporting(false)
      // 清空文件输入，允许重复选择同一文件
      if (fileInputRef.current) {
        fileInputRef.current.value = ''
      }
    }
  }

  const handleSelectFile = () => {
    fileInputRef.current?.click()
  }

  const handleToggleDisabled = async (id: number, currentDisabled: boolean) => {
    try {
      await setCredentialDisabled(id, !currentDisabled)
      refetch()
      toast.success(currentDisabled ? '已启用凭证' : '已禁用凭证')
      addLog(`[System] 凭证 #${id} ${currentDisabled ? '已启用' : '已禁用'}`)
    } catch (e) {
      toast.error('操作失败')
    }
  }

  const handleChangePriority = async (id: number, delta: number, currentPriority: number) => {
    const newPriority = Math.max(0, currentPriority + delta) // 不能小于 0
    if (newPriority === currentPriority) return // 已经是最小值
    try {
      await setCredentialPriority(id, newPriority)
      refetch()
      addLog(`[System] 凭证 #${id} 优先级已调整为 ${newPriority}`)
    } catch (e) {
      toast.error('操作失败')
    }
  }

  const handleResetFailure = async (id: number) => {
    try {
      await resetCredentialFailure(id)
      refetch()
      refreshBalance(id)
      toast.success('已重置并启用')
      addLog(`[System] 凭证 #${id} 已重置并启用`)
    } catch (e) {
      toast.error('操作失败')
    }
  }

  const handleDelete = async (id: number, isDisabled: boolean) => {
    // 检查是否已禁用
    if (!isDisabled) {
      toast.error('请先禁用凭证后再删除')
      return
    }
    
    if (!confirm('确定要删除此凭证吗？此操作不可撤销。')) return
    try {
      await deleteCredential(id)
      refetch()
      toast.success('已删除凭证')
      addLog(`[System] 凭证 #${id} 已删除`)
    } catch (e: any) {
      const message = e?.response?.data?.error?.message || '删除失败'
      toast.error(message)
      addLog(`[Error] 删除凭证 #${id} 失败: ${message}`)
    }
  }

  if (isLoading) {
    return (
      <div className="h-screen flex items-center justify-center bg-background">
        <div className="text-center">
          <div className="animate-spin rounded-full h-10 w-10 border-b-2 border-primary mx-auto mb-3"></div>
          <p className="text-muted-foreground text-sm">加载中...</p>
        </div>
      </div>
    )
  }

  if (error) {
    return (
      <div className="h-screen flex items-center justify-center bg-background p-4">
        <Card className="w-full max-w-sm">
          <CardContent className="pt-6 text-center">
            <div className="text-red-500 mb-3 text-sm">加载失败</div>
            <p className="text-muted-foreground mb-4 text-xs">{(error as Error).message}</p>
            <Button onClick={() => refetch()} size="sm">重试</Button>
          </CardContent>
        </Card>
      </div>
    )
  }

  return (
    <div className="h-screen flex bg-background overflow-hidden">
      {/* 左侧侧边栏 */}
      <aside className="w-56 border-r bg-muted/30 flex flex-col">
        {/* Logo */}
        <div className="h-14 flex items-center gap-2 px-4 border-b">
          <Server className="h-5 w-5 text-primary" />
          <span className="font-semibold">Kiro Gateway</span>
        </div>
        
        {/* 导航 */}
        <nav className="flex-1 p-3 space-y-1">
          <NavItem
            icon={<Server className="h-4 w-4" />}
            label="凭证管理"
            active={activeTab === 'credentials'}
            onClick={() => setActiveTab('credentials')}
          />
          <NavItem
            icon={<Settings className="h-4 w-4" />}
            label="反代配置"
            active={activeTab === 'config'}
            onClick={() => setActiveTab('config')}
          />
          <NavItem
            icon={<Terminal className="h-4 w-4" />}
            label="运行日志"
            active={activeTab === 'logs'}
            onClick={() => setActiveTab('logs')}
          />
        </nav>
        
        {/* 底部操作 */}
        <div className="p-3 border-t space-y-1">
          <button
            onClick={toggleDarkMode}
            className="w-full flex items-center gap-3 px-3 py-2 rounded-lg text-sm text-muted-foreground hover:text-foreground hover:bg-muted transition-colors"
          >
            {darkMode ? <Sun className="h-4 w-4" /> : <Moon className="h-4 w-4" />}
            {darkMode ? '浅色模式' : '深色模式'}
          </button>
        </div>
      </aside>

      {/* 主内容区 */}
      <main className="flex-1 flex flex-col overflow-hidden">
        {/* 顶栏 */}
        <header className="h-14 flex items-center justify-between px-6 border-b bg-background">
          <h1 className="text-lg font-semibold">
            {activeTab === 'credentials' && '凭证管理'}
            {activeTab === 'config' && '反代配置'}
            {activeTab === 'logs' && '运行日志'}
          </h1>
          <div className="flex items-center gap-2">
            {activeTab === 'credentials' && (
              <>
                <input
                  type="file"
                  ref={fileInputRef}
                  onChange={handleImportFile}
                  accept=".json"
                  className="hidden"
                />
                <Button variant="outline" size="sm" onClick={handleRefresh}>
                  <RefreshCw className="h-4 w-4 mr-1" />
                  刷新
                </Button>
                <Button variant="outline" size="sm" onClick={handleSelectFile} disabled={importing}>
                  {importing ? (
                    <>
                      <RefreshCw className="h-4 w-4 mr-1 animate-spin" />
                      导入中...
                    </>
                  ) : (
                    <>
                      <FolderOpen className="h-4 w-4 mr-1" />
                      导入凭证
                    </>
                  )}
                </Button>
                <Button size="sm" onClick={() => setAddDialogOpen(true)}>
                  <Plus className="h-4 w-4 mr-1" />
                  添加凭证
                </Button>
              </>
            )}
            {activeTab === 'config' && (
              <Button size="sm" onClick={handleSaveConfig} disabled={configSaving || configLoading}>
                {configSaving ? (
                  <>
                    <RefreshCw className="h-4 w-4 mr-1 animate-spin" />
                    保存中...
                  </>
                ) : (
                  <>
                    <Save className="h-4 w-4 mr-1" />
                    保存配置
                  </>
                )}
              </Button>
            )}
            {activeTab === 'logs' && (
              <Button variant="outline" size="sm" onClick={async () => {
                try {
                  const { clearLogs } = await import('@/api/credentials')
                  await clearLogs()
                  setLogs([])
                  setLocalLogs([])
                  toast.success('日志已清空')
                } catch (e) {
                  toast.error('清空日志失败')
                }
              }}>
                清空日志
              </Button>
            )}
          </div>
        </header>

        {/* 内容区 */}
        <div className="flex-1 overflow-auto p-6">
          {/* 凭证管理 */}
          {activeTab === 'credentials' && (
            <div className="space-y-4">
              {/* 统计 */}
              <div className="grid gap-4 grid-cols-3">
                <Card className="p-4">
                  <div className="text-xs text-muted-foreground mb-1">凭证总数</div>
                  <div className="text-2xl font-bold">{data?.total || 0}</div>
                </Card>
                <Card className="p-4">
                  <div className="text-xs text-muted-foreground mb-1">可用凭证</div>
                  <div className="text-2xl font-bold text-green-600">{data?.available || 0}</div>
                </Card>
                <Card className="p-4">
                  <div className="text-xs text-muted-foreground mb-1">当前活跃</div>
                  <div className="text-2xl font-bold">#{data?.currentId || '-'}</div>
                </Card>
              </div>

              {/* 表格 */}
              <Card>
                <div className="overflow-x-auto">
                  <table className="w-full text-sm">
                    <thead className="bg-muted/50">
                      <tr>
                        <th className="text-center px-4 py-3 font-medium">ID</th>
                        <th className="text-center px-4 py-3 font-medium">剩余额度</th>
                        <th className="text-center px-4 py-3 font-medium">Token有效期</th>
                        <th className="text-center px-4 py-3 font-medium">优先级</th>
                        <th className="text-center px-4 py-3 font-medium">状态</th>
                        <th className="text-center px-4 py-3 font-medium">失败次数</th>
                        <th className="text-center px-4 py-3 font-medium">操作</th>
                      </tr>
                    </thead>
                    <tbody className="divide-y">
                      {data?.credentials.length === 0 ? (
                        <tr>
                          <td colSpan={6} className="px-4 py-8 text-center text-muted-foreground">
                            暂无凭证
                          </td>
                        </tr>
                      ) : (
                        data?.credentials.map((cred) => (
                          <tr 
                            key={cred.id} 
                            className={`transition-colors ${cred.id === data.currentId 
                              ? 'bg-green-500/10 border-l-2 border-green-500 hover:bg-green-500/20' 
                              : 'hover:bg-muted/30'}`}
                          >
                            <td className="px-4 py-3 text-center">
                              <span className="font-mono">#{cred.id}</span>
                            </td>
                            <td className="px-4 py-3 text-center font-mono text-xs">
                              {cred.disabled ? (
                                <span className="text-muted-foreground">-</span>
                              ) : balances[cred.id]?.loading ? (
                                <span className="text-muted-foreground">加载中...</span>
                              ) : balances[cred.id]?.remaining === -1 ? (
                                <span className="text-red-400">获取失败</span>
                              ) : (
                                <span className={balances[cred.id]?.remaining < 1 ? 'text-red-500' : 'text-green-600'}>
                                  ${balances[cred.id]?.remaining?.toFixed(2) || '0.00'}
                                </span>
                              )}
                            </td>
                            <td className="px-4 py-3 text-center font-mono text-xs">
                              {cred.disabled ? (
                                <span className="text-muted-foreground">-</span>
                              ) : cred.expiresAt ? (
                                (() => {
                                  const expires = new Date(cred.expiresAt)
                                  const now = new Date()
                                  const diffMs = expires.getTime() - now.getTime()
                                  const diffMin = Math.floor(diffMs / 60000)
                                  
                                  if (diffMin < 0) {
                                    return <span className="text-red-500">已过期</span>
                                  } else if (diffMin < 10) {
                                    return <span className="text-yellow-500">{diffMin}分钟</span>
                                  } else if (diffMin < 60) {
                                    return <span className="text-green-500">{diffMin}分钟</span>
                                  } else {
                                    const hours = Math.floor(diffMin / 60)
                                    return <span className="text-green-600">{hours}小时</span>
                                  }
                                })()
                              ) : (
                                <span className="text-muted-foreground">未知</span>
                              )}
                            </td>
                            <td className="px-4 py-3 text-center">
                              <div className="flex items-center justify-center gap-1">
                                <button
                                  onClick={() => handleChangePriority(cred.id, -1, cred.priority)}
                                  className="p-1 hover:bg-muted rounded"
                                  title="提高优先级 (数值越小越优先)"
                                >
                                  <ChevronUp className="h-3 w-3" />
                                </button>
                                <span className="w-6 text-center">{cred.priority}</span>
                                <button
                                  onClick={() => handleChangePriority(cred.id, 1, cred.priority)}
                                  className="p-1 hover:bg-muted rounded"
                                  title="降低优先级"
                                >
                                  <ChevronDown className="h-3 w-3" />
                                </button>
                              </div>
                            </td>
                            <td className="px-4 py-3 text-center">
                              {cred.disabled ? (
                                <Badge variant="destructive" className="text-xs">已禁用</Badge>
                              ) : (
                                <Badge variant="success" className="text-xs">正常</Badge>
                              )}
                            </td>
                            <td className="px-4 py-3 text-center">
                              <span className={cred.failureCount > 0 ? 'text-red-500' : ''}>
                                {cred.failureCount}
                              </span>
                            </td>
                            <td className="px-4 py-3 text-center">
                              <div className="flex items-center justify-center gap-1">
                                <button
                                  onClick={() => handleToggleDisabled(cred.id, cred.disabled)}
                                  className="p-1.5 hover:bg-muted rounded"
                                  title={cred.disabled ? '启用' : '禁用'}
                                >
                                  {cred.disabled ? (
                                    <ToggleLeft className="h-4 w-4 text-muted-foreground" />
                                  ) : (
                                    <ToggleRight className="h-4 w-4 text-green-500" />
                                  )}
                                </button>
                                <button
                                  onClick={() => handleViewBalance(cred.id)}
                                  className="p-1.5 hover:bg-muted rounded"
                                  title="查看余额"
                                >
                                  <DollarSign className="h-4 w-4" />
                                </button>
                                <button
                                  onClick={() => handleResetFailure(cred.id)}
                                  className="p-1.5 hover:bg-muted rounded"
                                  title="重置并启用"
                                >
                                  <RotateCcw className="h-4 w-4" />
                                </button>
                                <button
                                  onClick={() => handleDelete(cred.id, cred.disabled)}
                                  className="p-1.5 hover:bg-muted rounded text-red-500"
                                  title={cred.disabled ? "删除凭证" : "请先禁用后再删除"}
                                >
                                  <Trash2 className="h-4 w-4" />
                                </button>
                              </div>
                            </td>
                          </tr>
                        ))
                      )}
                    </tbody>
                  </table>
                </div>
              </Card>
            </div>
          )}

          {/* 反代配置 */}
          {activeTab === 'config' && (
            <div className="space-y-4 max-w-2xl">


              <Card>
                <CardHeader className="pb-3">
                  <CardTitle className="text-sm flex items-center gap-2">
                    <Server className="h-4 w-4" />
                    反代服务
                  </CardTitle>
                </CardHeader>
                <CardContent>
                  <div className="grid gap-4 grid-cols-2">
                    <FormInput
                      label="监听地址"
                      value={configHost}
                      onChange={setConfigHost}
                      placeholder="127.0.0.1"
                      disabled={configLoading}
                    />
                    <FormInput
                      label="监听端口"
                      value={configPort}
                      onChange={setConfigPort}
                      type="number"
                      placeholder="8990"
                      disabled={configLoading}
                    />
                    <div className="col-span-2">
                      <FormInput
                        label="API 密钥"
                        value={configApiKey}
                        onChange={setConfigApiKey}
                        placeholder="sk-..."
                        disabled={configLoading}
                      />
                    </div>
                  </div>
                </CardContent>
              </Card>

              {/* API 端点 */}
              <Card>
                <CardHeader className="pb-3">
                  <CardTitle className="text-sm flex items-center gap-2">
                    <Terminal className="h-4 w-4" />
                    API 端点
                  </CardTitle>
                </CardHeader>
                <CardContent>
                  <div className="space-y-3 text-sm">
                    <div className="text-muted-foreground text-xs mb-2">以下端点可供外部调用（基于 http://{configHost}:{configPort}）</div>
                    
                    <div className="space-y-2">
                      <div className="font-medium text-xs text-muted-foreground">🔐 Anthropic API (需要 API Key)</div>
                      <div className="bg-muted rounded-lg p-3 space-y-2 text-xs">
                        <div className="flex justify-between items-center">
                          <code><span className="text-green-500">GET</span> /v1/models</code>
                          <span className="text-muted-foreground">获取可用模型列表</span>
                        </div>
                        <div className="flex justify-between items-center">
                          <code><span className="text-blue-500">POST</span> /v1/messages</code>
                          <span className="text-muted-foreground">创建对话 (流式/非流式)</span>
                        </div>
                        <div className="flex justify-between items-center">
                          <code><span className="text-blue-500">POST</span> /v1/messages/count_tokens</code>
                          <span className="text-muted-foreground">计算 Token 数量</span>
                        </div>
                      </div>
                    </div>

                    <div className="space-y-2">
                      <div className="font-medium text-xs text-muted-foreground">🔓 健康检查</div>
                      <div className="bg-muted rounded-lg p-3 space-y-2 text-xs">
                        <div className="flex justify-between items-center">
                          <code><span className="text-green-500">GET</span> / , /health , /ping</code>
                          <span className="text-muted-foreground">服务状态检查</span>
                        </div>
                      </div>
                    </div>

                  </div>
                </CardContent>
              </Card>
            </div>
          )}

          {/* 运行日志 */}
          {activeTab === 'logs' && (
            <Card className="h-full">
              <CardContent className="p-0 h-full">
                <div className="h-full overflow-y-auto bg-zinc-900 text-zinc-100 rounded-lg p-4 font-mono text-xs leading-relaxed">
                  {/* 本地日志 */}
                  {localLogs.map((log, index) => (
                    <div 
                      key={`local-${index}`} 
                      className={`py-0.5 ${
                        log.includes('[Error]') || log.includes('[ERROR]') ? 'text-red-400' : 
                        log.includes('[WARN]') ? 'text-yellow-400' :
                        log.includes('[System]') || log.includes('[INFO]') ? 'text-blue-400' : 
                        log.includes('[DEBUG]') ? 'text-zinc-500' :
                        'text-zinc-300'
                      }`}
                    >
                      {log}
                    </div>
                  ))}
                  {/* 后端日志 - 简洁模式 */}
                  {logs.length === 0 && localLogs.length === 0 ? (
                    <div className="text-zinc-500 text-center py-8">暂无日志</div>
                  ) : (
                    logs.map((log, index) => {
                      // 请求日志：显示用户提问摘要
                      if (log.request) {
                        const shortModel = log.request.model.replace('claude-', '').replace('-20251001', '').replace('-20251101', '')
                        const shortMsg = log.request.userMessagePreview.length > 50 
                          ? log.request.userMessagePreview.slice(0, 50) + '...'
                          : log.request.userMessagePreview
                        return (
                          <div key={`api-${index}`} className="py-0.5 text-green-400">
                            [{log.timestamp}] 📨 {shortModel} | {shortMsg}
                          </div>
                        )
                      }
                      // 响应日志：显示 token 消耗
                      if (log.response) {
                        const shortModel = log.response.model.replace('claude-', '').replace('-20251001', '').replace('-20251101', '')
                        return (
                          <div key={`api-${index}`} className="py-0.5 text-cyan-400">
                            [{log.timestamp}] 📤 {shortModel} | 输入: {log.response.inputTokens} | 输出: {log.response.outputTokens} | {log.response.stopReason}
                          </div>
                        )
                      }
                      // 其他日志
                      return (
                        <div key={`api-${index}`} className="py-0.5 text-zinc-400">
                          [{log.timestamp}] {log.message}
                        </div>
                      )
                    })
                  )}
                  <div ref={logsEndRef} />
                </div>
              </CardContent>
            </Card>
          )}
        </div>
      </main>

      {/* 对话框 */}
      <BalanceDialog
        credentialId={selectedCredentialId}
        open={balanceDialogOpen}
        onOpenChange={setBalanceDialogOpen}
      />
      <AddCredentialDialog
        open={addDialogOpen}
        onOpenChange={setAddDialogOpen}
      />
    </div>
  )
}
