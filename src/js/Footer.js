import {Component, html} from 'veda-client';

export default class Footer extends Component(HTMLElement) {
  static toString() {
    return 'bpa-footer'; 
  }

  render() {
    return html`
      <footer class="d-none d-lg-inline-block position-fixed bottom-0 end-0 text-end p-2 lh-1">
        <small><a class="me-1" href="https://semantic-machines.com">Смысловые машины.</a><a href="https://github.com/semantic-machines/veda">Платформа Veda</a></small>
      </footer>
    `;
  }
}
customElements.define(Footer.toString(), Footer);
