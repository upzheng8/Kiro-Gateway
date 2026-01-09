import { Dashboard } from '@/components/dashboard'
import { Toaster } from '@/components/ui/sonner'

function App() {
  return (
    <>
      <Dashboard onLogout={() => {}} />
      <Toaster position="top-right" />
    </>
  )
}

export default App

