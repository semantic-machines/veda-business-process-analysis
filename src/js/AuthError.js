import {Component, html} from 'veda-client';

export default class AuthError extends Component(HTMLElement) {
  static toString () {
    return 'bpa-auth-error';
  }

  pre () {
    this.root.querySelector('button').addEventListener('click', () => this.remove());
  }

  render () {
    return html`
      <div class="alert alert-danger alert-dismissible mt-4">
        ${this.errorDescription[this.getAttribute('code')]?.join(' / ')}
        <button type="button" class="btn-close" data-bs-dismiss="alert" aria-label="Close"></button>
      </div>
    `;
  }

  errorDescription = {
    '0': ['Отсутствует сетевое соединение с системой.', 'Network connection to the system is unavailable.'],
    '423': ['Пароль можно менять не чаще 1 раза в сутки.', 'Password may be changed only once a day.'],
    '429': ['Слишком много неудачных попыток аутентификации. Учетная запись заблокирована на 30 минут.', 'Too many failed authentication attempts. Account is locked for 30 minutes.'],
    '430': ['Слишком много неудачных попыток сменить пароль. Учетная запись заблокирована на 30 минут.', 'Too many failed password change attempts. Account is locked for 30 minutes.'],
    '463': ['Смена пароля для учетной записи запрещена.', 'Password change is not allowed.'],
    '464': ['Код просрочен.', 'Secret code expired.'],
    '465': ['Вы ввели пустой пароль.', 'You have entered empty password'],
    '466': ['Новый пароль совпадает с предыдущим.', 'New password is equal to previous.'],
    '467': ['Новый пароль не принят.', 'New password was not accepted.'],
    '468': ['Неверный код.', 'Invalid secret code.'],
    '469': ['Истекло время действия пароля. Вам выслан новый код для смены пароля. Пожалуйста, измените пароль, используя код.', 'Password expired. Secret code was sent to you. Please, change your password using secret code.'],
    '473': ['Неверное имя пользователя или пароль.', 'Wrong login or password.'],
  };
}

customElements.define(AuthError.toString(), AuthError);
