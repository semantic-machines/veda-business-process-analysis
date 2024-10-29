import {Component} from 'veda-client';

export default class Translate extends Component(HTMLButtonElement) {
  static toString () {
    return 'bpa-translate';
  }

  added () {
    document.documentElement.lang = localStorage.lang ?? document.documentElement.lang;
  }

  pre () {
    const languages = this.dataset.lang.split(/\W+/g);
    this.addEventListener('click', () => {
      let index = languages.indexOf(document.documentElement.lang);
      const lang = languages[++index % languages.length];
      document.documentElement.lang = lang;
      document.body.querySelectorAll('[property][lang]')?.forEach((el) => el.lang = lang);
      localStorage.lang = lang;
      this.update();
    }, {once: true});
  }

  render () {
    return document.documentElement.lang;
  }
}

customElements.define(Translate.toString(), Translate, {extends: 'button'});
