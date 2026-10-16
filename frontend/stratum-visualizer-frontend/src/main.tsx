import { render } from 'solid-js/web'
import { Shell } from './shell/shell'

const root = document.getElementById('root')
if (!root) {
  throw new Error('#root not found')
}
render(() => <Shell />, root)
