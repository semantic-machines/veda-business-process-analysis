import {Component, html} from 'veda-client';

export default class Footer extends Component(HTMLElement) {
  static tag = 'bpa-footer';

  render() {
    return html`
      <footer class="d-none d-lg-inline-block position-fixed bottom-0 end-0 text-end p-2 lh-1">
        <small><a class="me-1 text-dark" href="https://semantic-machines.com">© Смысловые машины.</a><a class="text-dark" href="https://github.com/semantic-machines/veda">Платформа Veda</a></small>
      </footer>
    `;
  }
}
customElements.define(Footer.tag, Footer);
