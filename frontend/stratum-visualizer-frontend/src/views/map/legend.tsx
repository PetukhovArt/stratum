import { Component } from 'solid-js'
import { t } from '../../i18n'

export const MapLegend: Component = () => (
  <div class="legend map-legend">
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
  </div>
)
