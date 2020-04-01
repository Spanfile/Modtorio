import React from 'react';
import { RouteComponentProps, Switch, Redirect, Route } from 'react-router';

interface IServerProps {
    serverId: string,
}

export default class Server extends React.Component<RouteComponentProps<IServerProps>> {
    public render(): JSX.Element {
        let props = this.props.match.params;
        let url = "/servers/" + props.serverId;
        return (
            <Switch>
                <Route path={url + "/status"} />
                <Route path={url + "/log"} />
                <Route path={url + "/configuration"} />
                <Redirect from={url} to={url + "/status"} />
            </Switch>
        );
    }
}