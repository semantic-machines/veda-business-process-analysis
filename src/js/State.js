import {Emitter, Observable} from 'veda-client';

const state = new (Observable(Emitter(Object)));

export default state;