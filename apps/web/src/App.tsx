import { useEffect, useState, type FormEvent } from 'react'
import type { AuthUserResponse, HealthResponse } from './generated/contracts'
import { fetchHealth, fetchMe, login, logout, postLabAction, register } from './lib/api'
import { FREE_MODE, LAB_MODE_OPTIONS } from './lib/challenges'
import { LabBackdrop } from './components/LabBackdrop'
import { LabBench } from './components/LabBench'

type Mode = 'login' | 'register'

export default function App() {
  const [health, setHealth] = useState<HealthResponse | null>(null)
  const [healthError, setHealthError] = useState<string | null>(null)
  const [user, setUser] = useState<AuthUserResponse | null>(null)
  const [mode, setMode] = useState<Mode>('login')
  const [email, setEmail] = useState('')
  const [displayName, setDisplayName] = useState('')
  const [password, setPassword] = useState('')
  const [authError, setAuthError] = useState<string | null>(null)
  const [busy, setBusy] = useState(false)
  const [sessionLoading, setSessionLoading] = useState(true)
  const [labMode, setLabMode] = useState(FREE_MODE)
  const [labModeEpoch, setLabModeEpoch] = useState(0)
  const [labModeBusy, setLabModeBusy] = useState(false)
  const [labModeError, setLabModeError] = useState<string | null>(null)
  const signupEnabled = health?.signup_enabled !== false

  useEffect(() => {
    let cancelled = false
    ;(async () => {
      try {
        const [h, me] = await Promise.all([fetchHealth(), fetchMe()])
        if (cancelled) return
        setHealth(h)
        setUser(me.user)
        if (!h.signup_enabled) {
          setMode('login')
        }
      } catch (err) {
        if (cancelled) return
        setHealthError(err instanceof Error ? err.message : 'API unreachable')
      } finally {
        if (!cancelled) setSessionLoading(false)
      }
    })()
    return () => {
      cancelled = true
    }
  }, [])

  async function onSubmit(event: FormEvent) {
    event.preventDefault()
    setAuthError(null)
    setBusy(true)
    try {
      const next =
        mode === 'register' && signupEnabled
          ? await register({
              email,
              password,
              display_name: displayName || email.split('@')[0] || 'Chemist',
            })
          : await login({ email, password })
      setUser(next)
      setPassword('')
    } catch (err) {
      setAuthError(err instanceof Error ? err.message : 'Authentication failed')
    } finally {
      setBusy(false)
    }
  }

  /** Switching mode is a hard reset on the server; remount the bench onto the new scene. */
  async function onSelectMode(next: string) {
    if (next === labMode || labModeBusy) return
    setLabModeBusy(true)
    setLabModeError(null)
    try {
      const response = await postLabAction({ type: 'select_mode', mode: next })
      setLabMode(response.scene.mode)
      setLabModeEpoch((epoch) => epoch + 1)
    } catch (err) {
      setLabModeError(err instanceof Error ? err.message : 'Could not switch mode')
    } finally {
      setLabModeBusy(false)
    }
  }

  async function onLogout() {
    setBusy(true)
    setAuthError(null)
    try {
      await logout()
      setUser(null)
    } catch (err) {
      setAuthError(err instanceof Error ? err.message : 'Logout failed')
    } finally {
      setBusy(false)
    }
  }

  return (
    <div className="relative min-h-screen overflow-x-hidden">
      <LabBackdrop />
      <header className="relative z-10 flex items-center justify-between px-5 py-4 sm:px-8">
        <span
          className="font-[family-name:var(--font-display)] text-lg font-semibold tracking-tight text-[var(--ink)]"
          aria-hidden
        >
          ChemLab
        </span>
        <span className="text-sm text-[var(--ink-soft)]">
          {health ? `API ${health.version}` : healthError ? 'API offline' : '…'}
        </span>
      </header>

      <main className="relative z-10 mx-auto flex min-h-[calc(100vh-4.5rem)] max-w-6xl flex-col gap-10 px-5 pb-16 pt-4 sm:px-8">
        <div className="grid items-center gap-10 lg:grid-cols-[1.15fr_0.85fr] lg:gap-14">
          <section className="animate-rise max-w-xl">
          <h1 className="font-[family-name:var(--font-display)] text-[clamp(3.4rem,10vw,5.6rem)] font-extrabold leading-[0.92] tracking-[-0.04em] text-[var(--ink)]">
            ChemLab
          </h1>
          <p className="animate-rise-delay mt-5 max-w-md text-lg leading-relaxed text-[var(--ink-soft)] sm:text-xl">
            Hands-on chemistry for the curious — scarce materials, real reactions, and a lab that
            remembers what you did.
          </p>
          <p className="mt-6 text-sm text-[var(--deep)]/80">
            Version 0.1 scaffold. The dissolve → evaporate loop lands in 1.0.
          </p>
        </section>

        <section
          className="animate-rise-delay w-full max-w-md justify-self-start lg:justify-self-end"
          aria-label="Account"
        >
          <div className="rounded-2xl border border-[var(--border)] bg-[var(--surface)] p-5 shadow-[var(--shadow)] backdrop-blur-md sm:p-6">
            {sessionLoading ? (
              <p className="text-[var(--ink-soft)]">Checking session…</p>
            ) : user ? (
              <div className="space-y-4">
                <p className="font-[family-name:var(--font-display)] text-2xl font-semibold tracking-tight">
                  Welcome back, {user.display_name}
                </p>
                <fieldset className="space-y-2" disabled={labModeBusy}>
                  <legend className="text-sm font-medium text-[var(--ink)]">Lab mode</legend>
                  {LAB_MODE_OPTIONS.map((option) => (
                    <label
                      key={option.id}
                      className="flex items-center gap-2 text-sm text-[var(--ink-soft)]"
                    >
                      <input
                        type="radio"
                        name="lab-mode"
                        value={option.id}
                        checked={labMode === option.id}
                        onChange={() => void onSelectMode(option.id)}
                      />
                      <span>{option.title}</span>
                    </label>
                  ))}
                </fieldset>

                {labModeError && (
                  <p className="text-sm text-[var(--danger)]" role="alert">
                    {labModeError}
                  </p>
                )}

                <button
                  type="button"
                  onClick={onLogout}
                  disabled={busy}
                  className="rounded-lg bg-[var(--ink)] px-4 py-2.5 text-sm font-semibold text-[var(--glass)] transition hover:opacity-90 disabled:opacity-60"
                >
                  {busy ? 'Signing out…' : 'Sign out'}
                </button>
              </div>
            ) : (
              <form className="space-y-4" onSubmit={onSubmit}>
                <div className="flex gap-2 text-sm">
                  {signupEnabled && (
                    <button
                      type="button"
                      className={`rounded-md px-3 py-1.5 font-medium ${
                        mode === 'register'
                          ? 'bg-[var(--ink)] text-[var(--glass)]'
                          : 'text-[var(--ink-soft)] hover:bg-[var(--surface-hover)]'
                      }`}
                      onClick={() => {
                        setMode('register')
                        setAuthError(null)
                      }}
                    >
                      Create account
                    </button>
                  )}
                  <button
                    type="button"
                    className={`rounded-md px-3 py-1.5 font-medium ${
                      mode === 'login'
                        ? 'bg-[var(--ink)] text-[var(--glass)]'
                        : 'text-[var(--ink-soft)] hover:bg-[var(--surface-hover)]'
                    }`}
                    onClick={() => {
                      setMode('login')
                      setAuthError(null)
                    }}
                  >
                    Sign in
                  </button>
                </div>

                {mode === 'register' && (
                  <label className="block space-y-1.5 text-sm">
                    <span className="font-medium text-[var(--ink)]">Display name</span>
                    <input
                      className="w-full rounded-lg border border-[var(--border)] bg-[var(--input-bg)] px-3 py-2 text-[var(--ink)] outline-none ring-[var(--accent)] placeholder:text-[var(--ink-soft)] focus:ring-2"
                      value={displayName}
                      onChange={(e) => setDisplayName(e.target.value)}
                      autoComplete="nickname"
                      placeholder="Player"
                    />
                  </label>
                )}

                <label className="block space-y-1.5 text-sm">
                  <span className="font-medium text-[var(--ink)]">Email</span>
                  <input
                    required
                    type="email"
                    className="w-full rounded-lg border border-[var(--border)] bg-[var(--input-bg)] px-3 py-2 text-[var(--ink)] outline-none ring-[var(--accent)] placeholder:text-[var(--ink-soft)] focus:ring-2"
                    value={email}
                    onChange={(e) => setEmail(e.target.value)}
                    autoComplete="email"
                    placeholder="you@lab.local"
                  />
                </label>

                <label className="block space-y-1.5 text-sm">
                  <span className="font-medium text-[var(--ink)]">Password</span>
                  <input
                    required
                    type="password"
                    minLength={8}
                    className="w-full rounded-lg border border-[var(--border)] bg-[var(--input-bg)] px-3 py-2 text-[var(--ink)] outline-none ring-[var(--accent)] placeholder:text-[var(--ink-soft)] focus:ring-2"
                    value={password}
                    onChange={(e) => setPassword(e.target.value)}
                    autoComplete={mode === 'register' ? 'new-password' : 'current-password'}
                    placeholder="At least 8 characters"
                  />
                </label>

                {authError && (
                  <p className="text-sm text-[var(--danger)]" role="alert">
                    {authError}
                  </p>
                )}

                <button
                  type="submit"
                  disabled={busy}
                  className="w-full rounded-lg bg-[var(--accent)] px-4 py-2.5 text-sm font-semibold text-[var(--glass)] transition hover:brightness-110 disabled:opacity-60"
                >
                  {busy ? 'Working…' : mode === 'register' ? 'Create account' : 'Sign in'}
                </button>

                <p className="text-xs leading-relaxed text-[var(--ink-soft)]">
                  Session cookie: HttpOnly <code>chemlab_session</code> plus CSRF
                  (<code>X-CSRF-Token</code>). Rate limits are next.
                </p>
              </form>
            )}
          </div>
        </section>
        </div>
        {user ? <LabBench key={labModeEpoch} onModeChange={setLabMode} /> : null}
      </main>
    </div>
  )
}
