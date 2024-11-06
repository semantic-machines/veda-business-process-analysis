import {Component, html} from 'veda-client';

export default class NotFound extends Component(HTMLElement) {
  static tag = 'bpa-not-found';

  render() {
    return html`
      <div class="container text-center mt-5">
        <h1 class="display-1">404</h1>
        <h2 class="mb-4">Страница не найдена</h2>
        <p class="lead mb-4">Извините, запрашиваемая страница не существует.</p>
        <a href="#/ProcessOverview" class="btn btn-secondary">
          <i class="bi bi-house"></i>
          Вернуться на главную
        </a>
      </div>
    `;
  }
}

customElements.define(NotFound.tag, NotFound);
