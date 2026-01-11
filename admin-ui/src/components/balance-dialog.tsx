import { useState } from 'react'
import { toast } from 'sonner'
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'
import { Progress } from '@/components/ui/progress'
import { Copy, Check } from 'lucide-react'
import { CredentialStatusItem } from '@/types/api'

interface BalanceDialogProps {
  credential: CredentialStatusItem | null
  open: boolean
  onOpenChange: (open: boolean) => void
}

export function BalanceDialog({ credential, open, onOpenChange }: BalanceDialogProps) {
  const [copied, setCopied] = useState<string | null>(null)

  const formatDate = (timestamp: number | null) => {
    if (!timestamp) return '未知'
    return new Date(timestamp * 1000).toLocaleString('zh-CN')
  }

  const formatNumber = (num: number | null) => {
    if (num === null) return '-'
    return num.toLocaleString('zh-CN', { minimumFractionDigits: 2, maximumFractionDigits: 2 })
  }

  const handleCopy = (text: string, field: string) => {
    navigator.clipboard.writeText(text)
    setCopied(field)
    toast.success('已复制')
    setTimeout(() => setCopied(null), 1500)
  }

  const maskToken = (token: string | null) => {
    if (!token || token.length < 20) return token || '-'
    return token.slice(0, 8) + '...' + token.slice(-8)
  }

  const usagePercentage = credential?.usageLimit 
    ? Math.min(100, ((credential.currentUsage || 0) / credential.usageLimit) * 100)
    : 0

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-md">
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            凭证 #{credential?.id}
            {credential?.subscriptionTitle && (
              <span className={`px-2 py-0.5 rounded text-xs font-medium ${
                credential.subscriptionTitle.includes('PRO+') 
                  ? 'bg-gradient-to-r from-purple-500 to-pink-500 text-white' 
                  : credential.subscriptionTitle.includes('PRO')
                    ? 'bg-blue-500 text-white'
                    : 'bg-muted text-muted-foreground'
              }`}>
                {credential.subscriptionTitle}
              </span>
            )}
          </DialogTitle>
        </DialogHeader>

        {credential ? (
          <div className="space-y-4">
            {/* 用户邮箱 */}
            {credential.email && (
              <div className="text-center text-muted-foreground text-sm">
                {credential.email}
              </div>
            )}

            {/* 配额信息 */}
            {credential.usageLimit !== null && (
              <div className="space-y-2">
                <div className="flex items-baseline justify-between">
                  <div>
                    <span className="text-lg font-bold">${formatNumber(credential.currentUsage)}</span>
                    <span className="text-muted-foreground text-sm ml-1">/ ${formatNumber(credential.usageLimit)}</span>
                  </div>
                  <span className={`text-xs font-medium ${
                    usagePercentage > 80 ? 'text-red-500' : 
                    usagePercentage > 50 ? 'text-yellow-600' : 'text-green-600'
                  }`}>
                    {usagePercentage.toFixed(1)}%
                  </span>
                </div>

                <Progress value={usagePercentage} className="h-1.5" />

                <div className="flex justify-between text-sm pt-1">
                  <span>
                    <span className="text-muted-foreground">剩余: </span>
                    <span className="font-medium text-green-600">${formatNumber(credential.remaining)}</span>
                  </span>
                  <span>
                    <span className="text-muted-foreground">重置: </span>
                    <span className="font-medium">{formatDate(credential.nextResetAt)}</span>
                  </span>
                </div>
              </div>
            )}

            {/* Token 信息 */}
            <div className="pt-3 border-t space-y-2 text-xs">
              {/* Refresh Token */}
              <div className="flex items-center justify-between">
                <span className="text-muted-foreground w-24">Refresh Token</span>
                <span className="font-mono flex-1 truncate mx-2">{maskToken(credential.refreshToken)}</span>
                {credential.refreshToken && (
                  <button 
                    onClick={() => handleCopy(credential.refreshToken!, 'refresh')}
                    className="text-muted-foreground hover:text-primary p-1"
                  >
                    {copied === 'refresh' ? <Check className="h-3 w-3 text-green-500" /> : <Copy className="h-3 w-3" />}
                  </button>
                )}
              </div>

              {/* Access Token */}
              <div className="flex items-center justify-between">
                <span className="text-muted-foreground w-24">Access Token</span>
                <span className="font-mono flex-1 truncate mx-2">{maskToken(credential.accessToken)}</span>
                {credential.accessToken && (
                  <button 
                    onClick={() => handleCopy(credential.accessToken!, 'access')}
                    className="text-muted-foreground hover:text-primary p-1"
                  >
                    {copied === 'access' ? <Check className="h-3 w-3 text-green-500" /> : <Copy className="h-3 w-3" />}
                  </button>
                )}
              </div>

              {/* Profile ARN */}
              {credential.profileArn && (
                <div className="flex items-center justify-between">
                  <span className="text-muted-foreground w-24">Profile ARN</span>
                  <span className="font-mono flex-1 truncate mx-2" title={credential.profileArn}>
                    {maskToken(credential.profileArn)}
                  </span>
                  <button 
                    onClick={() => handleCopy(credential.profileArn!, 'arn')}
                    className="text-muted-foreground hover:text-primary p-1"
                  >
                    {copied === 'arn' ? <Check className="h-3 w-3 text-green-500" /> : <Copy className="h-3 w-3" />}
                  </button>
                </div>
              )}
            </div>
          </div>
        ) : (
          <div className="py-6 text-center text-muted-foreground">
            请选择凭证
          </div>
        )}
      </DialogContent>
    </Dialog>
  )
}
