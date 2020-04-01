import React from 'react';
import { Switch, Route, Redirect } from 'react-router-dom';
import ServerView from './server/ServerView';

export default class AppView extends React.Component {
    public render(): JSX.Element {
        return (
            <Switch>
                <Route path="/servers" component={ServerView} />
                <Route path={"/settings"} />
                <Redirect exact from="/" to="servers" />
            </Switch>
        );
    }
}