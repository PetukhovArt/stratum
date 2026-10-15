import { JSX } from 'solid-js'
import { DesignData, DesignLayer, DesignModule, SeverityLabel, violationsOf } from '../../render/design'
import { layerColor, SEVERITY, TONE } from '../../render/palette'

const SAMPLE_SIZE = 3
const MONO = "'JetBrains Mono', monospace"

export const scopeViolationsTip = (
  data: DesignData,
  scope: { modules: DesignModule[]; label: string; severity: SeverityLabel },
): JSX.Element => {
  const rules = new Map<string, number>()
  const sample: { mod: string; msg: string }[] = []
  for (const m of scope.modules) {
    for (const v of violationsOf(data, m)) {
      if (v.severity !== scope.severity) continue
      rules.set(v.rule, (rules.get(v.rule) ?? 0) + 1)
      if (sample.length < SAMPLE_SIZE) {
        sample.push({ mod: m.label, msg: v.message })
      }
    }
  }
  if (rules.size === 0) return null
  const total = [...rules.values()].reduce((a, b) => a + b, 0)
  const sevClass = `sev-${scope.severity}`
  return (
    <>
      <div class="tt-hd">
        <span class={`tt-sev ${sevClass}`}>{scope.severity}</span>
        <span class="tt-name" style={{ margin: 0, 'font-size': '12px' }}>
          {scope.label}
        </span>
      </div>
      <div class="tt-vios" style={{ 'border-top': 'none', 'padding-top': 0 }}>
        {[...rules.entries()]
          .sort((a, b) => b[1] - a[1])
          .map(([rule, n]) => (
            <div class="tt-vio-row">
              <span class={`tt-vio-tag ${sevClass}`}>×{n}</span>
              <span class="tt-vio-rule">{rule}</span>
            </div>
          ))}
      </div>
      {sample.length > 0 && (
        <div
          style={{
            'margin-top': '8px',
            'padding-top': '8px',
            'border-top': '1px solid var(--bd-1)',
            display: 'flex',
            'flex-direction': 'column',
            gap: '4px',
          }}
        >
          {sample.map((s) => (
            <div style={{ 'font-size': '10.5px', color: 'var(--tx-3)' }}>
              <span style={{ 'font-family': MONO, color: 'var(--tx-2)' }}>{s.mod}</span>
              <span style={{ display: 'block', 'margin-top': '1px', 'line-height': 1.35 }}>
                {s.msg}
              </span>
            </div>
          ))}
          {total > sample.length && (
            <div class="tt-vio-more">+{total - sample.length} more — click row to inspect</div>
          )}
        </div>
      )}
    </>
  )
}

export const moduleViolationsTip = (data: DesignData, mod: DesignModule): JSX.Element => {
  const vios = violationsOf(data, mod)
  if (vios.length === 0) return null
  return (
    <>
      <div class="tt-hd">
        <span class={`tt-sev sev-${vios[0].severity}`}>{vios[0].severity}</span>
        <span class="tt-name" style={{ margin: 0, 'font-size': '12px', 'font-family': MONO }}>
          {mod.label}
        </span>
      </div>
      <div style={{ display: 'flex', 'flex-direction': 'column', gap: '6px' }}>
        {vios.map((v) => (
          <div
            style={{ 'border-left': `2px solid ${SEVERITY[v.severity].color}`, 'padding-left': '8px' }}
          >
            <div class="tt-vio-rule" style={{ 'margin-bottom': '2px' }}>
              {v.rule}
            </div>
            <div style={{ 'font-size': '11px', color: 'var(--tx-2)', 'line-height': 1.4 }}>
              {v.message}
            </div>
          </div>
        ))}
      </div>
    </>
  )
}

export const layerCountTip = (layer: DesignLayer): JSX.Element => (
  <>
    <div class="tt-hd">
      <span
        class="tt-layer"
        style={{
          background: layerColor(layer.hue, TONE.chipBg),
          color: layerColor(layer.hue, TONE.chipText),
        }}
      >
        {layer.label}
      </span>
    </div>
    <div class="tt-stats" style={{ 'margin-bottom': 0 }}>
      <span>{layer.modules} modules</span>
      {layer.errors > 0 && <span style={{ color: SEVERITY.error.color }}>{layer.errors} errors</span>}
      {layer.warnings > 0 && (
        <span style={{ color: SEVERITY.warning.color }}>{layer.warnings} warnings</span>
      )}
    </div>
  </>
)

export const nestedCountTip = (n: number): JSX.Element => (
  <div style={{ 'font-size': '11.5px', color: 'var(--tx-2)', 'line-height': 1.45 }}>
    <b style={{ color: 'var(--tx-1)' }}>
      {n} nested module{n === 1 ? '' : 's'}
    </b>
    <div style={{ 'margin-top': '4px', color: 'var(--tx-3)' }}>
      This module is a compound — click chevron to expand.
    </div>
  </div>
)
