import '@/App.css'
import { useEffect } from 'react'
import { RouterProvider } from '@tanstack/react-router'
import { router } from '@/router'
import { BenchmarkProvider } from '@/context/BenchmarkContext'
import { ThemeProvider, useTheme } from '@/context/ThemeContext'
import ErrorBoundary from '@/components/ErrorBoundary'

/**
 * Inner app component with theme sync capability
 * Listens for postMessage events from parent docs page to sync theme
 */
function AppContent(): React.ReactElement {
  const { setTheme } = useTheme()

  useEffect(() => {
    const handleThemeMessage = (event: MessageEvent) => {
      // Validate origin for security (allow production + dev environments)
      const allowedOrigins = [
        'https://kreuzberg.dev',
        'http://localhost',
        'http://127.0.0.1',
      ]
      const isAllowed = allowedOrigins.some((origin) => event.origin.startsWith(origin))

      if (!isAllowed) {
        return
      }

      // Handle theme change messages from parent
      if (event.data?.type === 'theme' && event.data?.value) {
        const theme = event.data.value
        if (theme === 'dark' || theme === 'light') {
          setTheme(theme)
        }
      }
    }

    window.addEventListener('message', handleThemeMessage)
    return () => window.removeEventListener('message', handleThemeMessage)
  }, [setTheme])

  return <RouterProvider router={router} />
}

/**
 * Main Application Component
 * Provides both the benchmark data context and router to the entire application.
 * Wrapped in ErrorBoundary to catch and gracefully handle any React component errors.
 * Also provides theme context for light/dark mode support with parent sync capability.
 */
function App(): React.ReactElement {
  return (
    <ErrorBoundary>
      <ThemeProvider>
        <BenchmarkProvider>
          <AppContent />
        </BenchmarkProvider>
      </ThemeProvider>
    </ErrorBoundary>
  )
}

export default App
