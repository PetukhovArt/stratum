import { Component } from 'solid-js'

export const Legend: Component = () => (
  <div class="legend">
    <div class="legend-row">
      <span class="legend-swatch sw-vio" />
      <span>violation</span>
    </div>
    <div class="legend-row">
      <span class="legend-glyph">◇</span>
      <span>composer</span>
    </div>
    <div class="legend-row">
      <span class="legend-glyph legend-glyph-s">_</span>
      <span>_shared (segment-private)</span>
    </div>
    <div class="legend-row">
      <span class="legend-thick" />
      <span>fractal (has children)</span>
    </div>
  </div>
)
