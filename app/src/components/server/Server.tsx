import React from 'react';
import { RouteComponentProps, Switch, Redirect, Route } from 'react-router';
import IServerProps from './IServerProps';
import General from './general/General';
import Log from './log/Log';

export default class Server extends React.Component<RouteComponentProps<IServerProps>> {
    public render(): JSX.Element {
        let props = this.props.match.params;
        let url = "/servers/" + props.serverId;
        return (
            <Switch>
                <Route path={url + "/general"} render={(p) => <General serverId={props.serverId} {...p} />} />
                <Route path={url + "/log"} render={(p) => <Log serverId={props.serverId} {...p} />} />
                <Route path={url + "/configuration"} />
                <Redirect from={url} to={url + "/general"} />
            </Switch>
        );
    }
}