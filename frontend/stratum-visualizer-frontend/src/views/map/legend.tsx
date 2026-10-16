import { Component, Show } from 'solid-js'
import { t } from '../../i18n'
import { MapVariant } from '../../shell/nav'

export const MapLegend: Component<{ variant: MapVariant }> = (props) => (
  <div class="legend map-legend">
    <Show
      when={props.variant === 'lanes'}
      fallback={
        <>
          <div class="legend-row">
            <span class="map-lg map-lg-line" />
            {t('map.legend.declared')}
          </div>
          <div class="legend-row">
            <span class="map-lg map-lg-line" />
            {t('map.legend.undeclared')}
          </div>
          <div class="legend-row">
            <span class="map-lg map-lg-sev" data-ch="sev" />
            {t('map.legend.finding')}
          </div>
        </>
      }
    >
      <div class="legend-row">
        <span class="map-lg map-lg-box" />
        {t('map.legend.unit')}
      </div>
      <div class="legend-row">
        <span class="map-lg map-lg-box dashed" />
        {t('map.legend.subUnit')}
      </div>
      <div class="legend-row">
        <span class="map-lg map-lg-line" />
        {t('map.legend.bundle')}
      </div>
      <div class="legend-row">
        <span class="map-lg map-lg-sev" data-ch="sev" />
        {t('map.legend.break')}
      </div>
      <div class="legend-row">
        <span class="map-lg" />
        {t('map.legend.hint')}
      </div>
    </Show>
  </div>
)
