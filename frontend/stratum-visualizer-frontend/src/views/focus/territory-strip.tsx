import { Component, For, Show } from 'solid-js'
import { t } from '../../i18n'
import { Model } from '../../render/model'
import { Forbidden } from '../../types'
import { layerName, layerTone } from './paint'

const ZONE_TAIL_SEGMENTS = 3

const forbiddenText = (m: Model, f: Forbidden): string => {
  if (f.region.kind === 'internals') return t('focus.terr.internals')
  const parent = m.containers.get(f.region.parent)?.name ?? ''
  return t(`focus.terr.siblings.${f.region.containerKind}`, { parent })
}

const forbiddenTitle = (m: Model, f: Forbidden): string => {
  if (f.region.kind === 'internals') return f.rule
  const paths = f.region.containers.map((id) => m.containers.get(id)?.path ?? String(id))
  return [f.rule, ...paths].join('\n')
}

export const TerritoryStrip: Component<{ model: Model; id: number }> = (props) => {
  const m = () => props.model
  const module = () => m().modules.get(props.id)!
  const territory = () => m().territoryOf(props.id)
  const excluded = () => m().layers.filter((l) => !territory().may_import_layers.includes(l.id))
  return (
    <div class="fo-terr" data-testid="territory-strip">
      <span class="fo-terr-k">{t('focus.terr.mayImport')}</span>
      <span class="fo-terr-v">
        <For each={territory().may_import_layers}>
          {(layer) => (
            <span
              class={`fo-chip${layer === module().layer ? ' self' : ''}`}
              title={layer === module().layer ? t('focus.terr.ownLayer') : undefined}
            >
              <span class="fo-sw" style={{ background: layerTone(m(), layer) }} />
              {layerName(m(), layer)}
            </span>
          )}
        </For>
        <span class="fo-n">{t('focus.terr.purity', { n: territory().max_import_purity })}</span>
      </span>
      <span class="fo-terr-k">{t('focus.terr.mayNot')}</span>
      <span class="fo-terr-v">
        <Show when={excluded().length > 0}>
          <span class="fo-chip no">{t('focus.terr.layers', { names: excluded().map((l) => l.name).join(', ') })}</span>
        </Show>
        <For each={territory().may_not}>
          {(f) => (
            <span class="fo-chip no" data-rule={f.rule} title={forbiddenTitle(m(), f)}>
              {forbiddenText(m(), f)}
            </span>
          )}
        </For>
        <Show when={excluded().length === 0 && territory().may_not.length === 0}>
          <span class="fo-n">{t('focus.terr.none')}</span>
        </Show>
      </span>
      <Show when={territory().visible_zones.length > 0}>
        <span class="fo-terr-k">{t('focus.terr.zones')}</span>
        <span class="fo-terr-v">
          <For each={territory().visible_zones}>
            {(zoneId) => {
              const zone = m().zones[zoneId]
              return (
                <span class="fo-chip zone" title={zone?.root}>
                  {zone?.path.split('/').slice(-ZONE_TAIL_SEGMENTS).join('/')}
                </span>
              )
            }}
          </For>
        </span>
      </Show>
      <span class="fo-terr-k">{t('focus.terr.importedBy')}</span>
      <span class="fo-terr-v fo-mono">{territory().imported_by_layers.map((l) => layerName(m(), l)).join(' · ')}</span>
    </div>
  )
}
