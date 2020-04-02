import React from 'react';
import { Button } from 'react-bootstrap';

export type StepComponentProps<T extends { [K in keyof T]?: any } = {}> = {
    next: () => void,
    cancel: () => void,
} & T

export type WizardProps = {
    initialStep?: number,
    done: () => void,
    children: React.ReactElement[];
}

export function Wizard(props: WizardProps) {
    const [step, setStep] = React.useState(props.initialStep ?? 0);

    let maxSteps = React.Children.count(props.children);
    let lastStep = maxSteps - 1;

    const nextStep = () => {
        if (step < lastStep) {
            return setStep(step + 1);
        }
    };

    let steps = React.Children.map(
        props.children,
        (step) => React.cloneElement(step, { next: nextStep, cancel: () => { console.log("cancel") } }));
    let currentStep = steps[step];

    return (
        <>
            {currentStep}
            <hr />
            <div className="d-flex flex-row-reverse">
                <Button
                    variant="primary"
                    className="ml-3"
                    onClick={nextStep}>
                    {step < lastStep ? "Next" : "Done"}
                </Button>
                <Button
                    variant="secondary"
                    className="ml-3">
                    Cancel
                </Button>
            </div>
        </>
    );
}

type StepProps = {
    render: (props: StepComponentProps<any>) => React.ReactNode;
}

export function Step<T extends StepProps = StepProps>(props: T) {
    return (
        <>
            {props.render({ ...props })}
        </>
    );
}
