import {Component, html} from 'veda-client';

export default class Error extends Component(HTMLElement) {
  static tag = 'bpa-error';

  render() {
    return html`
      <div class="container text-center mt-5">
        <h1 class="display-1">
          <i class="bi bi-exclamation-triangle text-danger"></i>
        </h1>
        <h2 class="mb-4">Произошла ошибка</h2>
        <p class="lead mb-4">Извините, что-то пошло не так. Пожалуйста, попробуйте позже.</p>
        <pre class="text-danger">${this.error?.message || 'Неизвестная ошибка'}</pre>
        <a href="#/ProcessOverview" class="btn btn-primary">
          <i class="bi bi-house"></i>
          Вернуться на главную
        </a>
      </div>
    `;
  }
}

customElements.define(Error.tag, Error);
