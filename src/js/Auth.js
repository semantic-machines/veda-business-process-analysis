import {Backend, Component, html} from 'veda-client';
import sha256 from 'tiny-sha256';

import App from './App.js';
import AuthError from './AuthError.js';

const show = (el) => el.style.display = 'block';
const hide = (el) => el.style.display = 'none';

export default class Auth extends Component(HTMLElement) {
  static tag = 'bpa-auth';

  async pre () {
    this.init();
    try {
      if (await Backend.is_ticket_valid()) {
        this.handleAuthSuccess();
      } else {
        this.handleAuthError();
      }
    } catch (error) {
      this.handleAuthError();
    }
  }

  handleAuthSuccess () {
    // Activity handler
    let lastActivity = Date.now();
    const activityHandler = () => {
      lastActivity = Date.now();
    };
    document.addEventListener('click', activityHandler);
    document.addEventListener('touchstart', activityHandler);

    const granted = Date.now();
    const expires = Backend.expires;
    const lifetime = expires > granted ? expires - granted : 0;
    const hh = Math.floor(lifetime / 1000 / 60 / 60);
    const mm = Math.floor((lifetime % (1000 * 60 * 60)) / 1000 / 60);
    const ss = Math.floor((lifetime % (1000 * 60)) / 1000);
    console.log(`Ticket will expire in ${hh < 10 ? '0' + hh : hh}:${mm < 10 ? '0' + mm : mm}:${ss < 10 ? '0' + ss : ss}`);
    const refreshInterval = setInterval(async () => {
      const expired = expires <= Date.now();
      const expiresSoon = expires - lifetime * 0.2 <= Date.now() && !expired;
      if (expired) {
        clearInterval(refreshInterval);
        console.log('Ticket expired, re-login.');
        await Backend.logout();
        this.handleAuthError();
      } else if (expiresSoon && granted < lastActivity) {
        clearInterval(refreshInterval);
        console.log('Refresh ticket in background.');
        document.removeEventListener('click', activityHandler);
        document.removeEventListener('touchstart', activityHandler);
        try {
          await Backend.get_ticket_trusted();
          this.handleAuthSuccess();
        } catch (error) {
          this.handleAuthError();
        }
      }
    }, 60 * 1000);

    document.querySelector(`${App}`).setAttribute('authenticated', '');
  }

  handleAuthError () {
    document.querySelector(`${App}`).removeAttribute('authenticated');
    this.handleLoginError();
  }

  render () {
    return html`
      <style scoped>
        #login-form {
          width: 80%;
          max-width: 30em;
          margin: 10% auto 0;
        }
      </style>
      <form id="login-form" name="login-form" role="form">
        <h3>Войдите / Sign in</h3>
        <div class="fieldset" id="enter-login-password">
          <input class="form-control form-control-lg" id="login" placeholder="Логин / login" type="text" name="login" autofocus="autofocus"/>
          <div class="input-group margin-lg">
            <input class="form-control form-control-lg password" id="password" placeholder="Пароль / password" type="password" name="password"/>
            <button class="btn btn-lg btn-outline-secondary show-password" type="button">&#128065;</button>
          </div>
          <button type="submit" id="submit-login-password" class="btn btn-lg btn-primary btn-block margin-lg">Войти / Sign in</button>
          <button type="button" id="change-password" class="btn btn-link btn-block btn-sm">Изменить пароль / Change password</button>
        </div>
        <div class="fieldset" id="enter-new-password">
          <div class="input-group">
            <input class="form-control form-control-lg password" id="new-password" placeholder="Новый пароль / New password" type="password" name="new-password" autocomplete="off"/>
            <button class="btn btn-lg btn-outline-secondary show-password" type="button">&#128065;</button>
          </div>
          <div class="input-group margin-lg">
            <input class="form-control form-control-lg password" id="confirm-new-password" placeholder="Повторите пароль / Confirm password" type="password" name="confirm-new-password" autocomplete="off"/>
            <button class="btn btn-lg btn-outline-secondary show-password" type="button">&#128065;</button>
          </div>
          <span class="help-block password-strength">Не менее 8 символов, должен содержать минимум 1 латинскую строчную букву, 1 латинскую прописную букву, 1 цифру</span>
          <span class="help-block password-strength">At least 8 characters and contain at least 1 lowercase latin letter, 1 uppercase latin letter, 1 digit</span>
          <span class="help-block password-must-match">Пароли должны совпадать</span>
          <span class="help-block password-must-match">Passwords must match</span>
          <input class="form-control input-lg margin-lg" id="secret" placeholder="Код из письма / Code from e-mail" type="text" name="secret" autocomplete="off"/>
          <span class="help-block enter-secret">Введите код из письма</span>
          <span class="help-block enter-secret">Enter secret form e-mail</span>
          <button disabled="disabled" type="submit" id="submit-new-password" class="btn btn-lg btn-primary btn-block margin-lg">Отправить / Submit</button>
        </div>
      </form>
    `;
  }

  init () {
    const submitLoginPassword = async (e) => {
      e.preventDefault();
      const passwordInput = loginForm.querySelector('#password');
      const login = loginForm.querySelector('#login').value;
      const password = passwordInput.value;
      const hash = sha256(password);

      passwordInput.value = '';

      try {
        const authResult = await Backend.authenticate(login, hash);
        this.handleLoginSuccess(authResult);
      } catch (error) {
        console.log(error);
        this.handleLoginError(error);
      }
    };

    const loginForm = this.root.lastChild;
    loginForm.addEventListener('submit', (e) => e.preventDefault());
    loginForm.querySelector('#submit-login-password').addEventListener('click', submitLoginPassword);
    loginForm.querySelectorAll('#login, #password').forEach((el) => el.addEventListener('keydown', (e) => {
      if (e.key === 'Enter') {
        e.preventDefault();
        submitLoginPassword(e);
        return false;
      }
    }));

    loginForm.querySelectorAll('.show-password').forEach((button) => {
      button.addEventListener('click', (e) => {
        const input = e.target.previousElementSibling;
        input.type = input.type === 'password' ? 'text' : 'password';
      });
    });

    const logoutHandler = async (e) => {
      if (e.target.matches('.logout')) {
        document.removeEventListener('click', logoutHandler);
        await Backend.logout().catch((error) => console.log('Logout failed', error));
        this.handleAuthError();
      }
    };
    document.addEventListener('click', logoutHandler);
  }

  handleLoginError (error) {
    const loginForm = this.root.lastChild;
    const enterLoginPassword = loginForm.querySelector('#enter-login-password');
    hide(enterLoginPassword);
    const enterNewPassword = loginForm.querySelector('#enter-new-password');
    hide(enterNewPassword);

    const inputs = loginForm.querySelectorAll('input:not(#login)');
    inputs.forEach((input) => input.value = '');

    if (error) {
      const authError = document.createElement(`${AuthError}`);
      authError.setAttribute('code', error.code);
      loginForm.appendChild(authError);
    }

    if (error?.code === '469') {
      show(enterNewPassword);
    } else {
      show(enterLoginPassword);
    }
  }

  handleLoginSuccess (auth) {
    const loginForm = this.root.lastChild;
    const inputs = loginForm.querySelectorAll('input:not(#login)');
    inputs.forEach((input) => input.value = '');
    this.handleAuthSuccess(auth);
  }
}

customElements.define(Auth.tag, Auth);
