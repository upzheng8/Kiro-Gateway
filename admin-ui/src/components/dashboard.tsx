import { useState, useEffect, useRef } from 'react'
import { RefreshCw, Moon, Sun, Server, Plus, Settings, Terminal, Save, Trash2, ToggleLeft, ToggleRight, Ghost, Eye, Info, Download, ChevronLeft, ChevronRight, ChevronDown, FolderOpen, FolderInput } from 'lucide-react'
import { useQueryClient } from '@tanstack/react-query'
import { toast } from 'sonner'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Button } from '@/components/ui/button'
import { Badge } from '@/components/ui/badge'
import { BalanceDialog } from '@/components/balance-dialog'
import { AddCredentialDialog } from '@/components/add-credential-dialog'
import { AboutDialog } from '@/components/about-dialog'
import { ExportDialog } from '@/components/export-dialog'
import { ConfirmDialog } from '@/components/confirm-dialog'
import { useCredentials } from '@/hooks/use-credentials'
import { setCredentialDisabled, deleteCredential } from '@/api/credentials'

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
  const [selectedCredential, setSelectedCredential] = useState<import('@/types/api').CredentialStatusItem | null>(null)
  const [balanceDialogOpen, setBalanceDialogOpen] = useState(false)
  const [addDialogOpen, setAddDialogOpen] = useState(false)
  const [aboutDialogOpen, setAboutDialogOpen] = useState(false)
  const [exportDialogOpen, setExportDialogOpen] = useState(false)
  const [activeTab, setActiveTab] = useState('credentials')
  
  // 删除确认对话框状态
  const [deleteConfirmOpen, setDeleteConfirmOpen] = useState(false)
  const [pendingDeleteId, setPendingDeleteId] = useState<number | null>(null)
  const [batchDeleteConfirmOpen, setBatchDeleteConfirmOpen] = useState(false)
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
  
  // 日志状态 - 使用 LogEntry 类型
  const [logs, setLogs] = useState<import('@/api/credentials').LogEntry[]>([])
  const [localLogs, setLocalLogs] = useState<string[]>(['[System] Kiro Gateway 已启动'])
  const logsEndRef = useRef<HTMLDivElement>(null)
  const logIntervalRef = useRef<ReturnType<typeof setInterval> | null>(null)

  useQueryClient() // keep hook call for potential future use
  const { data, isLoading, error, refetch } = useCredentials()
  
  // 多选状态
  const [selectedIds, setSelectedIds] = useState<Set<number>>(new Set());

  // 分页状态
  const [currentPage, setCurrentPage] = useState(1)
  const [pageSize, setPageSize] = useState(10)

  // 分组状态
  const [groups, setGroups] = useState<import('@/api/credentials').GroupInfo[]>([])
  const [activeGroupId, setActiveGroupId] = useState<string | null>(null)
  const [selectedGroupId, setSelectedGroupId] = useState<string>('default')
  const [groupsExpanded, setGroupsExpanded] = useState(true)
  const [addGroupDialogOpen, setAddGroupDialogOpen] = useState(false)
  const [newGroupName, setNewGroupName] = useState('')
  const [moveGroupDialogOpen, setMoveGroupDialogOpen] = useState(false)
  const [moveToGroupId, setMoveToGroupId] = useState('default')
  const [editGroupDialogOpen, setEditGroupDialogOpen] = useState(false)
  const [editingGroup, setEditingGroup] = useState<{id: string, name: string} | null>(null)
  const [editGroupName, setEditGroupName] = useState('')
  
  // 代理服务状态
  const [proxyRunning, setProxyRunning] = useState(true)

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

  // 获取分组列表
  useEffect(() => {
    const fetchGroups = async () => {
      try {
        const { getGroups, getProxyStatus } = await import('@/api/credentials')
        const response = await getGroups()
        setGroups(response.groups)
        setActiveGroupId(response.activeGroupId)
        
        // 获取代理状态
        try {
          const proxyStatus = await getProxyStatus()
          setProxyRunning(proxyStatus.running)
        } catch {}
      } catch (e) {
        // 忽略错误
      }
    }
    fetchGroups()
  }, [])



  useEffect(() => {
    logsEndRef.current?.scrollIntoView({ behavior: 'smooth' })
  }, [logs])

  const toggleDarkMode = () => {
    setDarkMode(!darkMode)
    document.documentElement.classList.toggle('dark')
  }

  const handleViewBalance = (cred: import('@/types/api').CredentialStatusItem) => {
    setSelectedCredential(cred)
    setBalanceDialogOpen(true)
  }

  const [isRefreshing, setIsRefreshing] = useState(false)
  const [refreshProgress, setRefreshProgress] = useState({ current: 0, total: 0, message: '' })
  
  // 添加凭证进度
  const [isImporting, setIsImporting] = useState(false)
  const [importProgress, setImportProgress] = useState({ current: 0, total: 0 })

  const handleRefresh = async () => {
    if (isRefreshing) return
    setIsRefreshing(true)
    
    const { refreshCredential } = await import('@/api/credentials')
    
    // 确定要刷新的凭证（只刷新当前分组的）
    const allCredentials = data?.credentials || []
    const filteredCredentials = allCredentials.filter(c => 
      selectedGroupId === 'all' || c.groupId === selectedGroupId
    )
    const idsToRefresh = selectedIds.size > 0 
      ? Array.from(selectedIds) 
      : filteredCredentials.filter(c => !c.disabled).map(c => c.id)
    
    const total = idsToRefresh.length
    let successCount = 0
    let failCount = 0
    
    setRefreshProgress({ current: 0, total, message: '准备刷新...' })
    addLog(`[System] 开始刷新 ${total} 个凭证...`)
    
    // 分批刷新，每批最多 10 个（并发数为 10）
    const batchSize = 10
    let completed = 0
    
    for (let i = 0; i < idsToRefresh.length; i += batchSize) {
      const batch = idsToRefresh.slice(i, i + batchSize)
      
      // 使用 Promise.all 并在每个完成时更新进度
      await Promise.all(
        batch.map(async (id) => {
          try {
            await refreshCredential(id)
            successCount++
          } catch {
            failCount++
          }
          completed++
          setRefreshProgress({ 
            current: completed, 
            total, 
            message: `刷新中 ${completed}/${total}` 
          })
        })
      )
    }
    
    // 刷新列表
    await refetch()
    
    setIsRefreshing(false)
    setRefreshProgress({ current: 0, total: 0, message: '' })
    
    if (failCount > 0) {
      toast.warning(`刷新完成: ${successCount} 成功, ${failCount} 失败`)
      addLog(`[System] 刷新完成: ${successCount} 成功, ${failCount} 失败`)
    } else {
      toast.success(`已刷新 ${successCount} 个凭证`)
      addLog(`[System] 已刷新 ${successCount} 个凭证`)
    }
  }

  // 刷新单个凭证
  const handleRefreshCredential = async (id: number) => {
    try {
      toast.info(`正在刷新凭证 #${id}...`)
      const { refreshCredential } = await import('@/api/credentials')
      const result = await refreshCredential(id)
      
      // 刷新列表
      await refetch()
      
      toast.success(result.message)
      addLog(`[System] ${result.message}`)
    } catch (e: any) {
      // 刷新失败也要刷新列表，因为后端已经更新了状态
      await refetch()
      
      const message = e?.response?.data?.error?.message || '刷新失败'
      toast.error(message)
      addLog(`[Error] 刷新凭证 #${id} 失败: ${message}`)
    }
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


  // 打开删除确认对话框
  const handleDeleteClick = (id: number) => {
    setPendingDeleteId(id)
    setDeleteConfirmOpen(true)
  }

  // 执行删除操作
  const handleConfirmDelete = async () => {
    if (pendingDeleteId === null) return
    try {
      await deleteCredential(pendingDeleteId)
      refetch()
      toast.success('已删除凭证')
      addLog(`[System] 凭证 #${pendingDeleteId} 已删除`)
    } catch (e: any) {
      const message = e?.response?.data?.error?.message || '删除失败'
      toast.error(message)
      addLog(`[Error] 删除凭证 #${pendingDeleteId} 失败: ${message}`)
    }
    setPendingDeleteId(null)
  }

  // 执行批量删除
  const handleConfirmBatchDelete = async () => {
    const ids = Array.from(selectedIds)
    try {
      const { batchDeleteCredentials } = await import('@/api/credentials')
      const result = await batchDeleteCredentials(ids)
      toast.success(result.message)
      setSelectedIds(new Set())
      refetch()
      addLog(`[System] 批量删除 ${ids.length} 个凭证成功`)
    } catch (e: any) {
      toast.error('删除失败')
      addLog(`[Error] 批量删除失败`)
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
      <aside className="w-45 border-r bg-muted/30 flex flex-col">
        {/* Logo */}
        <div className="h-14 flex items-center gap-2 px-4 border-b">
          <Server className="h-5 w-5 text-primary" />
          <span className="font-semibold">Kiro Gateway</span>
        </div>
        
        {/* 导航 */}
        <nav className="flex-1 p-3 space-y-1 overflow-y-auto">
          {/* 凭证管理 - 可折叠分组 */}
          <div>
            <button
              onClick={() => {
                if (activeTab === 'credentials') {
                  // 已经在凭证管理 tab，则切换折叠状态
                  setGroupsExpanded(!groupsExpanded)
                } else {
                  // 不在凭证管理 tab，则切换到该 tab（不改变折叠状态）
                  setActiveTab('credentials')
                }
              }}
              className={`w-full flex items-center justify-between gap-3 px-3 py-2 rounded-lg text-sm transition-colors ${
                activeTab === 'credentials'
                  ? 'bg-primary text-primary-foreground'
                  : 'text-muted-foreground hover:text-foreground hover:bg-muted'
              }`}
            >
              <div className="flex items-center gap-3">
                <Server className="h-4 w-4" />
                凭证管理
              </div>
              <ChevronDown className={`h-3 w-3 transition-transform ${groupsExpanded ? '' : '-rotate-90'}`} />
            </button>
            
            {/* 分组列表 */}
            {groupsExpanded && (
              <div className="mt-1 ml-3 space-y-0.5">
                {/* 全部分组 */}
                <button
                  onClick={() => {
                    setSelectedGroupId('all')
                    setActiveTab('credentials')
                    setSelectedIds(new Set())
                  }}
                  className={`w-full flex items-center gap-2 px-3 py-1.5 rounded text-xs transition-colors ${
                    selectedGroupId === 'all'
                      ? 'bg-muted text-foreground font-medium'
                      : 'text-muted-foreground hover:text-foreground hover:bg-muted/50'
                  }`}
                >
                  <FolderOpen className="h-3 w-3" />
                  全部
                  <span className="ml-auto text-[10px] text-muted-foreground">
                    {data?.total || 0}
                  </span>
                </button>
                
                {/* 各个分组 */}
                {groups.map(group => (
                  <button
                    key={group.id}
                    onClick={() => {
                      setSelectedGroupId(group.id)
                      setActiveTab('credentials')
                      setSelectedIds(new Set())
                    }}
                    onDoubleClick={() => {
                      if (group.id !== 'default') {
                        setEditingGroup(group)
                        setEditGroupName(group.name)
                        setEditGroupDialogOpen(true)
                      }
                    }}
                    className={`w-full flex items-center gap-2 px-3 py-1.5 rounded text-xs transition-colors ${
                      selectedGroupId === group.id
                        ? 'bg-muted text-foreground font-medium'
                        : 'text-muted-foreground hover:text-foreground hover:bg-muted/50'
                    }`}
                  >
                    <FolderOpen className="h-3 w-3" />
                    {group.name}
                    {activeGroupId === group.id && (
                      <span className="text-[10px] text-green-500" title="反代使用中">●</span>
                    )}
                    <span className="ml-auto text-[10px] text-muted-foreground">
                      {group.credentialCount}
                    </span>
                  </button>
                ))}
                
                {/* 添加分组按钮 */}
                <button
                  onClick={() => setAddGroupDialogOpen(true)}
                  className="w-full flex items-center gap-2 px-3 py-1.5 rounded text-xs text-muted-foreground hover:text-foreground hover:bg-muted/50 transition-colors"
                >
                  <Plus className="h-3 w-3" />
                  添加分组
                </button>
              </div>
            )}
          </div>
          
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
          <button
            onClick={() => setAboutDialogOpen(true)}
            className="w-full flex items-center gap-3 px-3 py-2 rounded-lg text-sm text-muted-foreground hover:text-foreground hover:bg-muted transition-colors"
          >
            <Info className="h-4 w-4" />
            关于
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
                {/* 刷新 */}
                <Button 
                  variant="outline" 
                  size="icon"
                  onClick={handleRefresh}
                  title={selectedIds.size > 0 ? `刷新(${selectedIds.size})` : '刷新'}
                >
                  <RefreshCw className="h-4 w-4" />
                </Button>
                
                {/* 选择后显示的批量操作按钮 */}
                {selectedIds.size > 0 && (
                  <>
                    {/* 导出 */}
                    <Button 
                      variant="outline" 
                      size="icon"
                      onClick={() => setExportDialogOpen(true)}
                      title={`导出(${selectedIds.size})`}
                    >
                      <Download className="h-4 w-4" />
                    </Button>
                    
                    {/* 删除 */}
                    <Button 
                      variant="outline" 
                      size="icon"
                      className="text-red-500 hover:text-red-600"
                      onClick={() => setBatchDeleteConfirmOpen(true)}
                      title={`删除(${selectedIds.size})`}
                    >
                      <Trash2 className="h-4 w-4" />
                    </Button>
                    
                    {/* 禁用 */}
                    <Button 
                      variant="outline" 
                      size="icon"
                      onClick={async () => {
                        const ids = Array.from(selectedIds)
                        try {
                          for (const id of ids) {
                            await setCredentialDisabled(id, true)
                          }
                          toast.success(`已禁用 ${ids.length} 个凭证`)
                          refetch()
                        } catch (e: any) {
                          toast.error('禁用失败')
                        }
                      }}
                      title={`禁用(${selectedIds.size})`}
                    >
                      <ToggleLeft className="h-4 w-4" />
                    </Button>
                    
                    {/* 启用 */}
                    <Button 
                      variant="outline" 
                      size="icon"
                      onClick={async () => {
                        const ids = Array.from(selectedIds)
                        try {
                          for (const id of ids) {
                            await setCredentialDisabled(id, false)
                          }
                          toast.success(`已启用 ${ids.length} 个凭证`)
                          refetch()
                        } catch (e: any) {
                          toast.error('启用失败')
                        }
                      }}
                      title={`启用(${selectedIds.size})`}
                    >
                      <ToggleRight className="h-4 w-4 text-green-500" />
                    </Button>
                    {/* 转移分组 */}
                    <Button
                      variant="outline"
                      size="icon"
                      onClick={() => setMoveGroupDialogOpen(true)}
                      title={`转移分组(${selectedIds.size})`}
                    >
                      <FolderInput className="h-4 w-4 text-blue-500" />
                    </Button>
                  </>
                )}
                
                {/* 添加凭证 */}
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
        <div className={`flex-1 p-6 ${activeTab === 'credentials' ? 'overflow-hidden flex flex-col' : 'overflow-auto'}`}>
          {/* 凭证管理 */}
          {activeTab === 'credentials' && (
            <div className="flex flex-col flex-1 gap-4 min-h-0">
              {/* 统计 */}
              <div className="grid gap-4 grid-cols-4 shrink-0">
                {(() => {
                  // 根据分组筛选
                  const filteredCreds = (data?.credentials || []).filter(c => 
                    selectedGroupId === 'all' || c.groupId === selectedGroupId
                  )
                  const total = filteredCreds.length
                  const available = filteredCreds.filter(c => !c.disabled && c.status === 'normal').length
                  const expired = filteredCreds.filter(c => c.status === 'expired').length
                  const invalid = filteredCreds.filter(c => c.status === 'invalid').length
                  
                  return (
                    <>
                      <Card className="p-4">
                        <div className="text-xs text-muted-foreground mb-1">凭证总数</div>
                        <div className="text-2xl font-bold">{total}</div>
                      </Card>
                      <Card className="p-4">
                        <div className="text-xs text-muted-foreground mb-1">可用凭证</div>
                        <div className="text-2xl font-bold text-green-600">{available}</div>
                      </Card>
                      <Card className="p-4">
                        <div className="text-xs text-muted-foreground mb-1">Token过期</div>
                        <div className="text-2xl font-bold text-yellow-600">{expired}</div>
                      </Card>
                      <Card className="p-4">
                        <div className="text-xs text-muted-foreground mb-1">无效/封禁</div>
                        <div className="text-2xl font-bold text-red-600">{invalid}</div>
                      </Card>
                    </>
                  )
                })()}
              </div>

              {/* 表格容器 */}
              <Card className="flex flex-col flex-1 min-h-0">
                {/* 表格主体 - 可滚动区域 */}
                <div className="flex-1 overflow-auto">
                  <table className="w-full text-sm">
                    <thead className="bg-card sticky top-0 z-10 border-b">
                      <tr>
                        <th className="text-center px-2 py-3 font-medium w-10">
                          <input 
                            type="checkbox" 
                            className="w-4 h-4 rounded"
                            checked={(() => {
                              const allCreds = (data?.credentials || []).filter(c => 
                                selectedGroupId === 'all' || c.groupId === selectedGroupId
                              )
                              const startIdx = (currentPage - 1) * pageSize
                              const pageData = allCreds.slice(startIdx, startIdx + pageSize)
                              return pageData.length > 0 && pageData.every(c => selectedIds.has(c.id))
                            })()}
                            onChange={(e) => {
                              const allCreds = (data?.credentials || []).filter(c => 
                                selectedGroupId === 'all' || c.groupId === selectedGroupId
                              )
                              const startIdx = (currentPage - 1) * pageSize
                              const pageData = allCreds.slice(startIdx, startIdx + pageSize)
                              if (e.target.checked) {
                                const newSet = new Set(selectedIds)
                                pageData.forEach(c => newSet.add(c.id))
                                setSelectedIds(newSet)
                              } else {
                                const newSet = new Set(selectedIds)
                                pageData.forEach(c => newSet.delete(c.id))
                                setSelectedIds(newSet)
                              }
                            }}
                          />
                        </th>
                        <th className="text-center px-4 py-3 font-medium">ID</th>
                        <th className="text-center px-4 py-3 font-medium">邮箱</th>
                        <th className="text-center px-4 py-3 font-medium">剩余额度</th>
                        <th className="text-center px-4 py-3 font-medium">Token有效期</th>
                        <th className="text-center px-4 py-3 font-medium">状态</th>
                        <th className="text-center px-4 py-3 font-medium">操作</th>
                      </tr>
                    </thead>
                    <tbody className="divide-y">
                      {(() => {
                        // 根据分组筛选凭证
                        const allCreds = (data?.credentials || []).filter(c => 
                          selectedGroupId === 'all' || c.groupId === selectedGroupId
                        )
                        const startIdx = (currentPage - 1) * pageSize
                        const pageData = allCreds.slice(startIdx, startIdx + pageSize)
                        
                        if (pageData.length === 0) {
                          return (
                            <tr>
                              <td colSpan={7} className="px-4 py-8 text-center text-muted-foreground">
                                暂无凭证
                              </td>
                            </tr>
                          )
                        }
                        
                        return pageData.map((cred) => (
                          <tr 
                            key={cred.id} 
                            className={`transition-colors ${cred.id === data?.currentId 
                              ? 'bg-green-500/10 hover:bg-green-500/20' 
                              : 'hover:bg-muted/30'}`}
                          >
                            <td className="px-2 py-3 text-center">
                              <input 
                                type="checkbox" 
                                className="w-4 h-4 rounded"
                                checked={selectedIds.has(cred.id)}
                                onChange={(e) => {
                                  const newSet = new Set(selectedIds)
                                  if (e.target.checked) {
                                    newSet.add(cred.id)
                                  } else {
                                    newSet.delete(cred.id)
                                  }
                                  setSelectedIds(newSet)
                                }}
                              />
                            </td>
                            <td className="px-4 py-3 text-center">
                              <span className="font-mono">#{cred.id}</span>
                            </td>
                            <td className="px-4 py-3 text-center text-xs">
                              {cred.email ? (
                                <span 
                                  className="cursor-default" 
                                  title={cred.email}
                                >
                                  {cred.email.replace(/(.{3}).*(@.*)/, '$1****$2')}
                                </span>
                              ) : (
                                <span className="text-muted-foreground">-</span>
                              )}
                            </td>
                            <td className="px-4 py-3 text-center font-mono text-xs">
                              {cred.disabled ? (
                                <span className="text-muted-foreground">-</span>
                              ) : cred.remaining !== null ? (
                                <span className={cred.remaining < 1 ? 'text-red-500' : 'text-green-600'}>
                                  ${cred.remaining.toFixed(2)}
                                </span>
                              ) : (
                                <span className="text-muted-foreground">-</span>
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
                              {(() => {
                                // 基于后端 status 字段显示状态
                                // disabled 单独处理（用户手动禁用）
                                if (cred.disabled) {
                                  return <Badge variant="secondary" className="text-xs">已禁用</Badge>
                                }
                                switch (cred.status) {
                                  case 'invalid':
                                    return <Badge variant="destructive" className="text-xs">无效</Badge>
                                  case 'expired':
                                    return <Badge variant="outline" className="text-xs text-yellow-600 border-yellow-600">已过期</Badge>
                                  default:
                                    return <Badge variant="success" className="text-xs">正常</Badge>
                                }
                              })()}
                            </td>
                            <td className="px-4 py-3 text-center">
                              <div className="flex items-center justify-center gap-1">
                                <button
                                  onClick={async () => {
                                    try {
                                      const { switchToCredential } = await import('@/api/credentials')
                                      const result = await switchToCredential(cred.id)
                                      toast.success(result.message)
                                      addLog(`[System] 已切换到凭证 #${cred.id}`)
                                    } catch (e: any) {
                                      toast.error(e.response?.data?.error?.message || '切换失败')
                                    }
                                  }}
                                  className="p-1.5 hover:bg-muted rounded"
                                  title="切换到此账号"
                                >
                                  <Ghost className="h-4 w-4" />
                                </button>
                                <button
                                  onClick={() => handleViewBalance(cred)}
                                  className="p-1.5 hover:bg-muted rounded"
                                  title="查看详情"
                                >
                                  <Eye className="h-4 w-4" />
                                </button>
                                <button
                                  onClick={() => handleRefreshCredential(cred.id)}
                                  className="p-1.5 hover:bg-muted rounded"
                                  title="刷新凭证"
                                >
                                  <RefreshCw className="h-4 w-4" />
                                </button>
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
                                  onClick={() => handleDeleteClick(cred.id)}
                                  className="p-1.5 hover:bg-muted rounded text-red-500"
                                  title="删除凭证"
                                >
                                  <Trash2 className="h-4 w-4" />
                                </button>
                              </div>
                            </td>
                          </tr>
                        ))
                      })()}
                    </tbody>
                  </table>
                </div>
                
                {/* 分页栏 - 固定在底部 */}
                <div className="border-t px-4 py-3 flex items-center justify-between text-sm shrink-0">
                  <div className="text-muted-foreground">
                    显示第 {Math.min((currentPage - 1) * pageSize + 1, data?.total || 0)} 到 {Math.min(currentPage * pageSize, data?.total || 0)} 条，共 {data?.total || 0} 条
                  </div>
                  <div className="flex items-center gap-4">
                    <div className="flex items-center gap-2">
                      <span className="text-muted-foreground">每页</span>
                      <select 
                        className="px-2 py-1 border rounded bg-background"
                        value={pageSize}
                        onChange={(e) => {
                          setPageSize(Number(e.target.value))
                          setCurrentPage(1)
                          setSelectedIds(new Set())
                        }}
                      >
                        <option value={10}>10</option>
                        <option value={20}>20</option>
                        <option value={50}>50</option>
                        <option value={100}>100</option>
                        <option value={200}>200</option>
                      </select>
                      <span className="text-muted-foreground">条</span>
                    </div>
                    <div className="flex items-center gap-2">
                      <button
                        onClick={() => {
                          setCurrentPage(p => Math.max(1, p - 1))
                          setSelectedIds(new Set())
                        }}
                        disabled={currentPage <= 1}
                        className="p-1 rounded hover:bg-muted disabled:opacity-50 disabled:cursor-not-allowed"
                      >
                        <ChevronLeft className="h-4 w-4" />
                      </button>
                      <span>{currentPage} / {Math.max(1, Math.ceil((data?.total || 0) / pageSize))}</span>
                      <button
                        onClick={() => {
                          setCurrentPage(p => Math.min(Math.ceil((data?.total || 0) / pageSize), p + 1))
                          setSelectedIds(new Set())
                        }}
                        disabled={currentPage >= Math.ceil((data?.total || 0) / pageSize)}
                        className="p-1 rounded hover:bg-muted disabled:opacity-50 disabled:cursor-not-allowed"
                      >
                        <ChevronRight className="h-4 w-4" />
                      </button>
                    </div>
                  </div>
                </div>
              </Card>
            </div>
          )}

          {/* 反代配置 */}
          {activeTab === 'config' && (
            <div className="space-y-4 max-w-2xl">


              <Card>
                <CardHeader className="pb-3">
                  <div className="flex items-center justify-between">
                    <CardTitle className="text-sm flex items-center gap-2">
                      <Server className="h-4 w-4" />
                      反代服务
                    </CardTitle>
                    {/* 启停开关 */}
                    <div className="flex items-center gap-2">
                      <span className="text-xs text-muted-foreground">
                        {proxyRunning ? '运行中' : '已停止'}
                      </span>
                      <div 
                        className={`w-10 h-5 rounded-full relative cursor-pointer transition-colors ${
                          proxyRunning ? 'bg-green-500' : 'bg-muted'
                        }`}
                        onClick={async () => {
                          try {
                            const { setProxyEnabled: setProxyEnabledApi } = await import('@/api/credentials')
                            await setProxyEnabledApi(!proxyRunning)
                            setProxyRunning(!proxyRunning)
                            toast.success(proxyRunning ? '代理服务已停止' : '代理服务已启动')
                          } catch (e: any) {
                            toast.error(e.response?.data?.error?.message || '操作失败')
                          }
                        }}
                      >
                        <div className={`absolute top-0.5 w-4 h-4 bg-white rounded-full shadow transition-all ${
                          proxyRunning ? 'left-5' : 'left-0.5'
                        }`} />
                      </div>
                    </div>
                  </div>
                </CardHeader>
                <CardContent>
                  <div className="space-y-4">
                    {/* 使用分组 - 放在最前面 */}
                    <div>
                      <label className="text-xs text-muted-foreground mb-1 block">使用分组</label>
                      <select
                        className="w-full px-3 py-2 bg-muted border border-border rounded-md text-sm focus:outline-none focus:ring-2 focus:ring-primary"
                        value={activeGroupId || 'all'}
                        onChange={async (e) => {
                          // "all" 表示全部分组，传 null 给后端
                          const selectedValue = e.target.value
                          const apiValue = selectedValue === 'all' ? null : selectedValue
                          try {
                            const { setActiveGroup, getGroups } = await import('@/api/credentials')
                            await setActiveGroup(apiValue)
                            setActiveGroupId(apiValue)
                            toast.success(apiValue ? `已切换到分组 "${groups.find(g => g.id === apiValue)?.name}"` : '已切换到所有分组')
                            // 刷新分组列表
                            const response = await getGroups()
                            setGroups(response.groups)
                          } catch (e: any) {
                            toast.error(e.response?.data?.error?.message || '切换失败')
                          }
                        }}
                      >
                        <option value="all">全部分组</option>
                        {groups.map(group => (
                          <option key={group.id} value={group.id}>
                            {group.name} ({group.credentialCount})
                          </option>
                        ))}
                      </select>
                      <p className="text-xs text-muted-foreground mt-1">
                        选择反代服务使用的凭证分组，必须选择后才能启用服务
                      </p>
                    </div>
                    
                    {/* 其他配置 */}
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

              {/* 机器码管理 */}
              <Card>
                <CardHeader className="pb-3">
                  <CardTitle className="text-sm flex items-center gap-2">
                    <Settings className="h-4 w-4" />
                    机器码管理
                  </CardTitle>
                </CardHeader>
                <CardContent>
                  <div className="space-y-3">
                    <div className="text-xs text-muted-foreground">
                      机器码用于识别客户端设备，可备份后在其他设备恢复。
                    </div>
                    <div className="flex gap-2">
                      <Button 
                        variant="outline" 
                        size="sm" 
                        onClick={async () => {
                          try {
                            const { backupMachineId } = await import('@/api/credentials')
                            const result = await backupMachineId()
                            toast.success(result.message)
                            addLog('[System] 机器码已备份')
                          } catch (e: any) {
                            toast.error(e.response?.data?.error?.message || '备份失败')
                          }
                        }}
                      >
                        备份机器码
                      </Button>
                      <Button 
                        variant="outline" 
                        size="sm" 
                        onClick={async () => {
                          try {
                            const { restoreMachineId } = await import('@/api/credentials')
                            const result = await restoreMachineId()
                            toast.success(result.message)
                            addLog('[System] 机器码已恢复')
                          } catch (e: any) {
                            toast.error(e.response?.data?.error?.message || '恢复失败')
                          }
                        }}
                      >
                        恢复机器码
                      </Button>
                      <Button 
                        variant="destructive" 
                        size="sm" 
                        onClick={async () => {
                          if (!confirm('确定要重置机器码吗？这将生成新的设备标识。')) return
                          try {
                            const { resetMachineId } = await import('@/api/credentials')
                            const result = await resetMachineId()
                            toast.success(result.message)
                            addLog('[System] 机器码已重置')
                          } catch (e: any) {
                            toast.error(e.response?.data?.error?.message || '重置失败')
                          }
                        }}
                      >
                        重置机器码
                      </Button>
                    </div>
                  </div>
                </CardContent>
              </Card>

              {/* 模型锁定 */}
              <Card>
                <CardHeader className="pb-3">
                  <CardTitle className="text-sm flex items-center gap-2">
                    <Settings className="h-4 w-4" />
                    模型锁定（仅客户端）
                  </CardTitle>
                </CardHeader>
                <CardContent>
                  <div className="space-y-3">
                    <div className="text-xs text-muted-foreground">
                      锁定后客户端操作将使用指定模型，不影响反代功能。
                    </div>
                    <div className="flex gap-2">
                      <select 
                        className="flex-1 px-3 py-2 bg-muted border border-border rounded-md text-sm"
                        defaultValue=""
                        onChange={async (e) => {
                          try {
                            const { setLockedModel } = await import('@/api/credentials')
                            const model = e.target.value || null
                            const result = await setLockedModel(model)
                            toast.success(result.message)
                            addLog(`[System] 模型锁定: ${model || '已取消'}`)
                          } catch (err: any) {
                            toast.error(err.response?.data?.error?.message || '设置失败')
                          }
                        }}
                      >
                        <option value="">不锁定</option>
                        <option value="claude-sonnet-4-20250514">Claude Sonnet 4</option>
                        <option value="claude-3-5-sonnet-20241022">Claude 3.5 Sonnet</option>
                        <option value="claude-3-5-haiku-20241022">Claude 3.5 Haiku</option>
                        <option value="claude-opus-4-20250514">Claude Opus 4</option>
                      </select>
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
        credential={selectedCredential}
        open={balanceDialogOpen}
        onOpenChange={setBalanceDialogOpen}
      />
      <AddCredentialDialog
        open={addDialogOpen}
        onOpenChange={setAddDialogOpen}
        onImportStart={() => setIsImporting(true)}
        onImportProgress={(current, total) => setImportProgress({ current, total })}
        selectedGroupId={selectedGroupId}
        groups={groups}
        onImportEnd={() => {
          setIsImporting(false)
          setImportProgress({ current: 0, total: 0 })
          refetch()
        }}
      />
      <AboutDialog
        open={aboutDialogOpen}
        onOpenChange={setAboutDialogOpen}
      />
      <ExportDialog
        open={exportDialogOpen}
        onOpenChange={setExportDialogOpen}
        selectedIds={Array.from(selectedIds)}
      />

      {/* 删除确认对话框 */}
      <ConfirmDialog
        open={deleteConfirmOpen}
        onOpenChange={setDeleteConfirmOpen}
        title="删除凭证"
        description="确定要删除此凭证吗？此操作不可撤销。"
        onConfirm={handleConfirmDelete}
        confirmText="删除"
        variant="destructive"
      />

      {/* 批量删除确认对话框 */}
      <ConfirmDialog
        open={batchDeleteConfirmOpen}
        onOpenChange={setBatchDeleteConfirmOpen}
        title="批量删除凭证"
        description={`确定要删除选中的 ${selectedIds.size} 个凭证吗？此操作不可撤销。`}
        onConfirm={handleConfirmBatchDelete}
        confirmText="删除"
        variant="destructive"
      />

      {/* 全局进度遮罩 */}
      {isRefreshing && refreshProgress.total > 0 && (
        <div className="fixed inset-0 bg-black/50 backdrop-blur-sm z-50 flex items-center justify-center">
          <div className="bg-card p-8 rounded-xl shadow-2xl text-center min-w-[300px]">
            <div className="animate-spin rounded-full h-12 w-12 border-4 border-primary border-t-transparent mx-auto mb-4"></div>
            <div className="text-2xl font-bold text-foreground mb-2">
              {refreshProgress.current} / {refreshProgress.total}
            </div>
            <div className="text-muted-foreground">
              {refreshProgress.message}
            </div>
            <div className="mt-4 h-2 bg-muted rounded-full overflow-hidden">
              <div 
                className="h-full bg-primary transition-all duration-300"
                style={{ width: `${(refreshProgress.current / refreshProgress.total) * 100}%` }}
              />
            </div>
          </div>
        </div>
      )}

      {/* 导入凭证进度遮罩 */}
      {isImporting && importProgress.total > 0 && (
        <div className="fixed inset-0 bg-black/50 backdrop-blur-sm z-50 flex items-center justify-center">
          <div className="bg-card p-8 rounded-xl shadow-2xl text-center min-w-[300px]">
            <div className="animate-spin rounded-full h-12 w-12 border-4 border-primary border-t-transparent mx-auto mb-4"></div>
            <div className="text-2xl font-bold text-foreground mb-2">
              {importProgress.current} / {importProgress.total}
            </div>
            <div className="text-muted-foreground">
              正在添加凭证...
            </div>
            <div className="mt-4 h-2 bg-muted rounded-full overflow-hidden">
              <div 
                className="h-full bg-primary transition-all duration-300"
                style={{ width: `${(importProgress.current / importProgress.total) * 100}%` }}
              />
            </div>
          </div>
        </div>
      )}

      {/* 添加分组对话框 */}
      {addGroupDialogOpen && (
        <div 
          className="fixed inset-0 bg-black/50 flex items-center justify-center z-50"
          onClick={() => {
            setAddGroupDialogOpen(false)
            setNewGroupName('')
          }}
        >
          <div className="bg-background border rounded-lg p-6 w-80" onClick={(e) => e.stopPropagation()}>
            <h3 className="text-lg font-semibold mb-4">添加分组</h3>
            <input
              type="text"
              placeholder="分组名称"
              value={newGroupName}
              onChange={(e) => setNewGroupName(e.target.value)}
              className="w-full px-3 py-2 border rounded-md mb-4 bg-background"
              autoFocus
            />
            <div className="flex justify-end gap-2">
              <Button
                variant="outline"
                onClick={() => {
                  setAddGroupDialogOpen(false)
                  setNewGroupName('')
                }}
              >
                取消
              </Button>
              <Button
                onClick={async () => {
                  if (!newGroupName.trim()) {
                    toast.error('请输入分组名称')
                    return
                  }
                  try {
                    const { addGroup: addGroupApi, getGroups } = await import('@/api/credentials')
                    await addGroupApi(newGroupName.trim())
                    toast.success('分组创建成功')
                    // 刷新分组列表
                    const response = await getGroups()
                    setGroups(response.groups)
                    setAddGroupDialogOpen(false)
                    setNewGroupName('')
                  } catch (e) {
                    toast.error('创建分组失败')
                  }
                }}
              >
                创建
              </Button>
            </div>
          </div>
        </div>
      )}

      {/* 转移分组对话框 */}
      {moveGroupDialogOpen && (
        <div 
          className="fixed inset-0 bg-black/50 flex items-center justify-center z-50"
          onClick={() => {
            setMoveGroupDialogOpen(false)
            setMoveToGroupId('default')
          }}
        >
          <div className="bg-background border rounded-lg p-6 w-80" onClick={(e) => e.stopPropagation()}>
            <h3 className="text-lg font-semibold mb-4">转移分组</h3>
            <p className="text-sm text-muted-foreground mb-4">
              将选中的 {selectedIds.size} 个凭证转移到：
            </p>
            <select
              className="w-full px-3 py-2 border rounded-md mb-4 bg-background"
              value={moveToGroupId}
              onChange={(e) => setMoveToGroupId(e.target.value)}
            >
              {groups.map(group => (
                <option key={group.id} value={group.id}>
                  {group.name}
                </option>
              ))}
            </select>
            <div className="flex justify-end gap-2">
              <Button
                variant="outline"
                onClick={() => {
                  setMoveGroupDialogOpen(false)
                  setMoveToGroupId('default')
                }}
              >
                取消
              </Button>
              <Button
                onClick={async () => {
                  try {
                    const { setCredentialGroup } = await import('@/api/credentials')
                    let successCount = 0
                    for (const id of selectedIds) {
                      try {
                        await setCredentialGroup(id, moveToGroupId)
                        successCount++
                      } catch {}
                    }
                    toast.success(`已转移 ${successCount} 个凭证`)
                    setMoveGroupDialogOpen(false)
                    setMoveToGroupId('default')
                    setSelectedIds(new Set())
                    refetch()
                    // 刷新分组列表
                    const { getGroups } = await import('@/api/credentials')
                    const response = await getGroups()
                    setGroups(response.groups)
                  } catch (e) {
                    toast.error('转移失败')
                  }
                }}
              >
                确定
              </Button>
            </div>
          </div>
        </div>
      )}

      {/* 编辑分组对话框 */}
      {editGroupDialogOpen && editingGroup && (
        <div 
          className="fixed inset-0 bg-black/50 flex items-center justify-center z-50"
          onClick={() => {
            setEditGroupDialogOpen(false)
            setEditingGroup(null)
            setEditGroupName('')
          }}
        >
          <div className="bg-background border rounded-lg p-6 w-80" onClick={(e) => e.stopPropagation()}>
            <h3 className="text-lg font-semibold mb-4">编辑分组</h3>
            <div className="space-y-4">
              <div>
                <label className="text-sm font-medium">分组名称</label>
                <input
                  type="text"
                  value={editGroupName}
                  onChange={(e) => setEditGroupName(e.target.value)}
                  className="w-full px-3 py-2 border rounded-md mt-1 bg-background"
                />
              </div>
              <div className="text-xs text-muted-foreground">
                双击分组可编辑。默认分组不可编辑或删除。
              </div>
            </div>
            <div className="flex justify-between mt-6">
              <Button
                variant="destructive"
                size="sm"
                onClick={async () => {
                  if (!editingGroup) return
                  try {
                    const { deleteGroup, getGroups } = await import('@/api/credentials')
                    await deleteGroup(editingGroup.id)
                    toast.success('分组已删除')
                    setEditGroupDialogOpen(false)
                    setEditingGroup(null)
                    setEditGroupName('')
                    // 如果删除的是当前选中的分组，切换到 all
                    if (selectedGroupId === editingGroup.id) {
                      setSelectedGroupId('all')
                    }
                    // 刷新分组列表
                    const response = await getGroups()
                    setGroups(response.groups)
                    refetch()
                  } catch (e: any) {
                    toast.error(e.response?.data?.error?.message || '删除失败')
                  }
                }}
              >
                <Trash2 className="h-4 w-4 mr-1" />
                删除
              </Button>
              <div className="flex gap-2">
                <Button
                  variant="outline"
                  onClick={() => {
                    setEditGroupDialogOpen(false)
                    setEditingGroup(null)
                    setEditGroupName('')
                  }}
                >
                  取消
                </Button>
                <Button
                  onClick={async () => {
                    if (!editingGroup) return
                    if (!editGroupName.trim()) {
                      toast.error('请输入分组名称')
                      return
                    }
                    try {
                      const { renameGroup, getGroups } = await import('@/api/credentials')
                      await renameGroup(editingGroup.id, editGroupName.trim())
                      toast.success('分组已重命名')
                      setEditGroupDialogOpen(false)
                      setEditingGroup(null)
                      setEditGroupName('')
                      // 刷新分组列表
                      const response = await getGroups()
                      setGroups(response.groups)
                    } catch (e: any) {
                      toast.error(e.response?.data?.error?.message || '重命名失败')
                    }
                  }}
                >
                  保存
                </Button>
              </div>
            </div>
          </div>
        </div>
      )}
    </div>
  )
}
