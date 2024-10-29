import {Component, html, Backend} from 'veda-client';
import Translate from './Translate.js';

export default class Header extends Component(HTMLElement) {
  static toString() {
    return 'bpa-header';
  }
  
  render() {
    return html`
      <header>
        <nav class="navbar navbar-expand-lg p-0" style="height:80px;">
          <div class="container px-3">
            <a class="navbar-brand position-relative me-3 p-0" href="#/portal:Main"><img src="images/semantic-machines-logo-long.svg" alt="logo" style="height:32px;"></a>
            <a class="navbar-brand position-relative me-3 p-0" href="#/portal:Main"><img src="images/optiflow.png" alt="logo" style="height:32px;"></a>
            <button class="navbar-toggler" type="button" data-bs-toggle="offcanvas" data-bs-target="#offcanvasNavbar" aria-controls="offcanvasNavbar">
              <span class="navbar-toggler-icon"></span>
            </button>
            <div class="offcanvas offcanvas-end" tabindex="-1" id="offcanvasNavbar" aria-labelledby="offcanvasNavbarLabel">
              <div class="offcanvas-header">
                <h5 class="offcanvas-title" id="offcanvasNavbarLabel">Меню</h5>
                <button type="button" class="btn-close" data-bs-dismiss="offcanvas" aria-label="Close"></button>
              </div>
              <div class="offcanvas-body justify-content-end align-items-center">
                <ul class="navbar-nav">
                  <li class="nav-item">
                    <button class="btn btn-outline-secondary me-4" is="${Translate}" data-lang="ru, en"></button>
                  </li>
                  <li class="nav-item">
                    <span class="nav-link" about="${Backend.user}" property="rdfs:label"></span>
                  </li>
                  <li class="nav-item">
                    <button class="btn btn-outline-secondary logout bi bi-box-arrow-right"></button>
                  </li>
                </ul>
              </div>
            </div>
          </div>
        </nav>
      </header>
    `;
  }
}
customElements.define(Header.toString(), Header);
